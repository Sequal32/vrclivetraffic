mod adsbexchange;
mod airports;
mod error;
mod flightaware;
mod flightradar;
mod interpolate;
mod noaa;
mod request;
mod tracker;
mod util;

use airports::Airports;
use flightaware::FlightPlan;
use fsdparser::{ClientQueryPayload, PacketTypes, Parser};
use log::{info, LevelFilter};
use serde::{Deserialize, Serialize};
use simplelog::{Config, TermLogger, TerminalMode};
use std::collections::{hash_map::Entry, HashMap};
use std::fmt::Display;
use std::fs::File;
use std::io::{Read, Write};
use std::net::TcpListener;
use std::rc::Rc;
use std::thread::sleep;
use std::time::Instant;
use tracker::{TrackData, Tracker};
use util::BoxedData;

const CONFIG_FILENAME: &str = "config.json";
const AIRPORT_DATA_FILENAME: &str = "airports.dat";

fn build_aircraft_string(data: &mut TrackData, should_interpolate: bool) -> String {
    let ac_data = &data.ac_data;
    // Calculate PBH
    let h = ac_data.heading() as f64 / 360.0 * 1024.0;
    let pbh: i32 = 0 << 22 | 0 << 12 | (h as i32) << 2;

    let pos = if should_interpolate {
        data.position.get()
    } else {
        data.position.get_no_update()
    };

    format!(
        "@N:{callsign}:{squawk}:1:{lat}:{lon}:{alt}:{speed}:{pbh}:0\r\n",
        callsign = ac_data.callsign(),
        squawk = ac_data.squawk(),
        lat = pos.lat,
        lon = pos.lon,
        alt = ac_data.altitude(),
        speed = ac_data.ground_speed(),
        pbh = pbh
    )
}

fn get_remarks(ac_data: &BoxedData) -> String {
    format!("Provider {}, Hex {}", ac_data.provider(), ac_data.hex())
}

fn build_flightplan_string(fp: &FlightPlan, ac_data: &BoxedData) -> String {
    let fp_remarks = format!(
        "{}{}{}{}",
        fp.departure_time
            .as_ref()
            .map(|x| format!(
                "STD {}, ",
                x.scheduled.naive_utc().format("%H%MZ").to_string()
            ))
            .unwrap_or_default(),
        fp.arrival_time
            .as_ref()
            .map(|x| format!(
                "STA {}, ",
                x.scheduled.naive_utc().format("%H%MZ").to_string()
            ))
            .unwrap_or_default(),
        fp.destination
            .gate
            .as_ref()
            .map(|x| format!("Departure Gate {}, ", x))
            .unwrap_or_default(),
        fp.origin
            .gate
            .as_ref()
            .map(|x| format!("Arrival Gate {}", x))
            .unwrap_or_default(),
    );

    format!("$FP{callsign}::I:{equipment}:{speed}:{origin}:0:0:{altitude}:{destination}:0:0:0:0::/v/ {remarks}:{route}\r\n",
        callsign = ac_data.callsign(),
        equipment = fp.equipment.ac_type,
        speed = fp.fp.speed,
        origin = fp.origin.icao,
        altitude = fp.fp.altitude,
        destination = fp.destination.icao,
        route = fp.fp.route,
        remarks = get_remarks(ac_data) + ", " + &fp_remarks
    )
}

fn build_init_flightplan_string(ac_data: &BoxedData, is_airline: bool) -> String {
    format!(
        "$FP{callsign}::{flight_rules}:{equipment}:0:{origin}:0:0:0:{destination}:0:0:0:0::/v/ {remarks}:\r\n",
        flight_rules = if is_airline {"I"} else {"V"},
        callsign = ac_data.callsign(),
        equipment = ac_data.model(),
        origin = ac_data.origin(),
        destination = ac_data.destination(),
        remarks = get_remarks(ac_data)
    )
}

fn build_beacon_code_string(my_callsign: &str, callsign: &str, beacon_code: &str) -> String {
    format!(
        "#PCSERVER:{}:CCP:BC:{}:{}\r\n",
        my_callsign, callsign, beacon_code
    )
}

fn build_metar_string(callsign: &str, metar: &String) -> String {
    format!("$ARSERVER:{}:METAR:{}\r\n", callsign, metar)
}

fn build_validate_atc_string_with_callsign(callsign: &str) -> String {
    format!("$CRSERVER:{0:}:ATC:Y:{0:}\r\n", callsign)
}

fn build_validate_atc_string_without_callsign(callsign: &str) -> String {
    format!("$CRSERVER:{0:}:ATC:Y\r\n", callsign)
}

#[derive(Default)]
struct TrackedData {
    did_set_fp: bool,
    did_init_set: bool,
}

#[derive(Deserialize, Serialize)]
struct ConfigData {
    airport: String,
    range: u32,
    delay: u64,
    floor: i32,
    ceiling: i32,
    use_flightaware: bool,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            airport: String::new(),
            range: 30,
            delay: 0,
            floor: 0,
            ceiling: 99999,
            use_flightaware: true,
        }
    }
}

fn read_config() -> Result<ConfigData, std::io::Error> {
    let file = File::open(CONFIG_FILENAME)?;
    Ok(serde_json::from_reader(file)?)
}

fn display_msg_and_exit(msg: impl Display) {
    println!("{}\nPress the enter key to exit.", msg);
    // Wait for enter key
    let mut buf = String::new();
    std::io::stdin().read_line(&mut buf).ok();
    std::process::exit(0);
}

fn main() {
    // Setup logging
    TermLogger::init(LevelFilter::Info, Config::default(), TerminalMode::Stdout).ok();
    // Bind TCP server
    let listener = match TcpListener::bind("127.0.0.1:6809") {
        Ok(l) => l,
        Err(e) => {
            display_msg_and_exit(format!("Could not start server! Reason: {}", e));
            return;
        }
    };

    // Read from config
    let config: ConfigData = match read_config() {
        Ok(config) => config,
        Err(_) => {
            let config = ConfigData::default();
            serde_json::to_writer_pretty(File::create(CONFIG_FILENAME).unwrap(), &config).ok();

            display_msg_and_exit("Could not read config.json! Please set an airport ICAO and range in the newly created file.");

            config
        }
    };

    // Load airports
    let airports = match Airports::new(AIRPORT_DATA_FILENAME) {
        Ok(a) => Rc::new(a),
        Err(e) => {
            display_msg_and_exit(format!("Could not read airports.dat! Reason: {}", e));
            return;
        }
    };

    let bounds = match airports.get_bounds_from_radius(&config.airport, config.range as f32) {
        Some(b) => {
            info!("Airport set to {}", config.airport);
            b
        }
        None => {
            display_msg_and_exit(format!("The airport {} does not exist!", config.airport));
            return;
        }
    };

    // Weather
    let weather = noaa::NoaaWeather::new();
    weather.run();

    loop {
        info!("Waiting for connection...");

        let (mut stream, addr) = listener.accept().unwrap();

        info!("Connection established! {}", addr.to_string());

        // Confirms connection with connect
        stream
            .write("$DISERVER:CLIENT:VATSIM FSD V3.14:\r\n".as_bytes())
            .ok();

        stream.set_nonblocking(true).ok();

        // Instantiate main tracker
        let mut tracker = Tracker::new(&bounds, airports.clone(), config.floor, config.ceiling);
        // Start loops to listen for data
        if config.use_flightaware {
            tracker.run_faware();
        }

        // Map to keep track of data already injected
        let mut injected_tracker: HashMap<String, TrackedData> = HashMap::new();
        let mut current_callsign = String::new();
        let mut timer: Option<Instant> = None;
        let buffer_timer = Instant::now();

        info!("Displaying aircraft...");

        tracker.start_buffering();

        'main: loop {
            tracker.step();

            let should_update_position =
                timer.is_none() || timer.unwrap().elapsed().as_secs_f32() >= 5.0;

            let ac_data = tracker.get_aircraft_data();
            let aircraft_count = ac_data.len();
            for aircraft in ac_data {
                // Insert aircraft as "injected" if not already in
                let tracked: &mut TrackedData = match injected_tracker.entry(aircraft.id.clone()) {
                    Entry::Occupied(o) => o.into_mut(),
                    Entry::Vacant(v) => v.insert(TrackedData::default()),
                };
                // Update position either in place or interpolated
                if should_update_position {
                    let should_interpolate = !aircraft.ac_data.is_on_ground()
                        && aircraft.at_last_position_update.elapsed().as_secs() < 20;
                    if let Err(_) =
                        stream.write(build_aircraft_string(aircraft, should_interpolate).as_bytes())
                    {
                        break 'main;
                    };

                    // Give the aircraft an initial flight plan
                    if !tracked.did_init_set && !aircraft.fp.is_some() {
                        stream
                            .write(
                                build_init_flightplan_string(
                                    &aircraft.ac_data,
                                    aircraft.is_airline,
                                )
                                .as_bytes(),
                            )
                            .ok();
                        tracked.did_init_set = true;
                    }

                    // Give the aircraft a flight plan if available
                    if !tracked.did_set_fp && aircraft.fp.is_some() {
                        stream
                            .write(
                                build_flightplan_string(
                                    aircraft.fp.as_ref().unwrap(),
                                    &aircraft.ac_data,
                                )
                                .as_bytes(),
                            )
                            .ok();
                        // Not squawking anything... will have duplicates if we assign an empty code
                        if aircraft.ac_data.squawk() != "0000" {
                            stream
                                .write(
                                    build_beacon_code_string(
                                        &current_callsign,
                                        &aircraft.ac_data.callsign(),
                                        &aircraft.ac_data.squawk(),
                                    )
                                    .as_bytes(),
                                )
                                .ok();
                        }

                        tracked.did_set_fp = true;
                    }
                }
            }

            if should_update_position {
                // AKA 3 second intervals
                // Reset position update timer
                timer = Some(Instant::now());

                // Manage buffering
                if tracker.is_buffering() {
                    let elaspsed = buffer_timer.elapsed().as_secs();
                    if elaspsed >= config.delay {
                        tracker.stop_buffering();
                    } else {
                        info!(
                            "Buffering... {} seconds left to buffer.",
                            (config.delay - elaspsed).max(0)
                        );
                    }
                } else {
                    info!("Updating aircraft: {} shown.", aircraft_count);
                }
            }

            // Remove dropped off radar aircraft
            for id in injected_tracker
                .keys()
                .map(|x| x.clone())
                .collect::<Vec<String>>()
            {
                if tracker.aircraft_exists(&id) {
                    continue;
                }
                injected_tracker.remove(&id);
            }

            // Accept/Parse data from client
            let mut client_request = String::new();

            stream.read_to_string(&mut client_request).ok();

            if client_request.trim() != "" {
                for request in client_request.split("\n") {
                    let packet = match Parser::parse(request) {
                        Some(p) => p,
                        None => continue,
                    };

                    match packet {
                        PacketTypes::Metar(metar) => {
                            if metar.is_response {
                                continue;
                            }

                            info!("Getting weather for {}", metar.payload);
                            weather.request_weather(&metar.payload)
                        }
                        PacketTypes::ClientQuery(cq) => match cq.payload {
                            ClientQueryPayload::FlightPlan(callsign) => {
                                let data = match tracker.get_data_for_callsign(&callsign) {
                                    Some(d) => d,
                                    None => continue,
                                };

                                let fp = match &data.fp {
                                    Some(fp) => fp,
                                    None => continue,
                                };

                                stream
                                    .write(build_flightplan_string(fp, &data.ac_data).as_bytes())
                                    .ok();
                            }
                            ClientQueryPayload::IsValidATCQuery(callsign) => {
                                // Recognize callsign as a valid controller
                                current_callsign = cq.from.to_string();
                                info!("Validating {} as an ATC", current_callsign);
                                // Some ATC clients handle validating ATC differently
                                if callsign.is_some() {
                                    stream
                                        .write(
                                            build_validate_atc_string_with_callsign(
                                                &current_callsign,
                                            )
                                            .as_bytes(),
                                        )
                                        .ok();
                                } else {
                                    stream
                                        .write(
                                            build_validate_atc_string_without_callsign(
                                                &current_callsign,
                                            )
                                            .as_bytes(),
                                        )
                                        .ok();
                                }
                            }
                            _ => (),
                        },
                        _ => (),
                    }
                }
            }

            // Step stuff
            if let Some(Ok(metar)) = weather.get_next_weather() {
                info!("Got metar {}", metar);
                stream
                    .write(build_metar_string(&current_callsign, &metar).as_bytes())
                    .ok();
            }

            sleep(std::time::Duration::from_millis(10));
        }
    }
}
