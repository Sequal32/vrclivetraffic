mod adsbexchange;
mod airports;
mod error;
mod interpolate;
mod flightradar;
mod flightaware;
mod noaa;
mod request;
mod tracker;
mod util;

use airports::Airports;
use flightaware::FlightPlan;
use fsdparser::{Parser, PacketTypes, ClientQueryPayload};
use serde::{Deserialize, Serialize};
use std::{collections::{HashMap, hash_map::Entry}, fmt::Display, fs::File, io::Read, io::Write, net::TcpListener, time::Instant};
use tracker::{TrackData, Tracker};

fn build_aircraft_string(data: &mut TrackData, should_interpolate: bool) -> String {
    let ac_data = &data.data;
    // Calculate PBH
    let h = ac_data.heading as f64/360.0 * 1024.0;
    let pbh: i32 = 0 << 22 | 0 << 12 | (h as i32) << 2;

    let pos = if should_interpolate {data.position.get()} else {data.position.get_no_update()};

    format!("@N:{callsign}:{squawk}:1:{lat}:{lon}:{alt}:{speed}:{pbh}:0\r\n", 
        callsign = ac_data.callsign,
        squawk = ac_data.squawk,
        lat = pos.lat,
        lon = pos.lon,
        alt = ac_data.altitude,
        speed = ac_data.ground_speed,
        pbh = pbh
    )
}

fn build_flightplan_string(callsign: &str, fp: &FlightPlan) -> String {
    format!("$FP{callsign}::I:{equipment}:{speed}:{origin}:0:0:{altitude}:{destination}:0:0:0:0::/v/:{route}\r\n",
        callsign = callsign,
        equipment = fp.equipment,
        speed = fp.speed,
        origin = fp.origin,
        altitude = fp.altitude,
        destination = fp.destination,
        route = fp.route
    )
}

fn build_beacon_code_string(my_callsign: &str, callsign: &str, beacon_code: &str) -> String {
    format!("#PCSERVER:{}:CCP:BC:{}:{}\r\n", my_callsign, callsign, beacon_code)
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
    did_set_fp: bool
}

#[derive(Deserialize, Serialize)]
struct ConfigData {
    airport: String,
    range: u32,
    delay: u64,
    floor: i32,
    ceiling: i32,
}

impl Default for ConfigData {
    fn default() -> Self {
        Self {
            airport: String::new(),
            range: 30,
            delay: 0,
            floor: 0,
            ceiling: 99999,
        }
    }
}

const CONFIG_FILENAME: &str = "config.json";
const AIRPORT_DATA_FILENAME: &str = "airports.dat";

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
    let listener = match TcpListener::bind("127.0.0.1:6809") {
        Ok(l) => l,
        Err(e) => {display_msg_and_exit(format!("Could not start server! Reason: {}", e)); return;}
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
        Ok(airports) => airports,
        Err(e) => {display_msg_and_exit(format!("Could not read airports.dat! Reason: {}", e)); return;}
    };

    let bounds = match airports.get_bounds_from_radius(&config.airport, config.range as f32) {
        Some(b) => b,
        None => {display_msg_and_exit(format!("The airport {} does not exist!", config.airport)); return;}
    };

    // Weather
    let weather = noaa::NoaaWeather::new();
    weather.run();

    loop {
        println!("Waiting for connection...");
        
        let (mut stream, addr) = listener.accept().unwrap();

        println!("Connection established! {}", addr.to_string());

        // Confirms connection with connect
        stream.write("$DISERVER:CLIENT:VATSIM FSD V3.14:\r\n".as_bytes()).ok();

        stream.set_nonblocking(true).ok();

        // Instantiate main tracker
        let mut tracker = Tracker::new(&bounds, config.floor, config.ceiling);
        // Start loops to listen for data
        tracker.run();

        // Map to keep track of data already injected
        let mut injected_tracker: HashMap<String, TrackedData> = HashMap::new();
        let mut current_callsign = String::new();
        let mut timer = Instant::now();
        let buffer_timer = Instant::now();
        
        println!("Displaying aircraft...");
        
        tracker.start_buffering();

        'main: loop {
            tracker.step();

            let should_update_position = timer.elapsed().as_secs_f32() >= 5.0;

            let ac_data = tracker.get_aircraft_data();
            let aircraft_count = ac_data.len();
            for aircraft in ac_data {
                // Insert aircraft as "injected" if not already in
                let tracked: &mut TrackedData = match injected_tracker.entry(aircraft.id.clone()) {
                    Entry::Occupied(o) => o.into_mut(),
                    Entry::Vacant(v) => v.insert(TrackedData::default())
                };
                // Update position either in place or interpolated
                if should_update_position {
                    let should_interpolate = !aircraft.data.is_on_ground && aircraft.at_last_position_update.unwrap().elapsed().as_secs() < 20;
                    if let Err(_) = stream.write(build_aircraft_string(aircraft, should_interpolate).as_bytes()) {break 'main};
                }
                // Give the aircraft a flight plan 
                if !tracked.did_set_fp && aircraft.fp.is_some() {
                    stream.write(build_flightplan_string(&aircraft.data.callsign, aircraft.fp.as_ref().unwrap()).as_bytes()).ok();
                    // Not squawking anything... will have duplicates if we assign an empty code
                    if aircraft.data.squawk != "0000" {
                        stream.write(build_beacon_code_string(&current_callsign, &aircraft.data.callsign, &aircraft.data.squawk).as_bytes()).ok();
                    }
                    
                    tracked.did_set_fp = true;
                }
            }

            if should_update_position { // AKA 3 second intervals
                // Reset position update timer
                timer = Instant::now();

                 // Manage buffering
                if tracker.is_buffering() {
                    let elaspsed = buffer_timer.elapsed().as_secs();
                    if elaspsed >= config.delay {
                        tracker.stop_buffering();
                    }
                    else {
                        println!("Buffering... {} seconds left to buffer.", (config.delay-elaspsed).max(0));
                    }
                }
                else {
                    println!("Updating aircraft: {} shown.", aircraft_count);
                }
            }

            // Remove dropped off radar aircraft
            for id in injected_tracker.keys().map(|x| x.clone()).collect::<Vec<String>>() {
                if tracker.aircraft_exists(&id) {continue}
                injected_tracker.remove(&id);
            }

            // Accept/Parse data from client
            let mut client_request = String::new();
            stream.read_to_string(&mut client_request).ok();
            if client_request.trim() != "" {
                for request in client_request.split("\n") {
                    match Parser::parse(request) {
                        Some(packet) => match packet {
                            PacketTypes::Metar(metar) => {
                                if !metar.is_response {
                                    weather.request_weather(&metar.payload)
                                }
                            },
                            PacketTypes::ClientQuery(cq) => match cq.payload {
                                ClientQueryPayload::FlightPlan(callsign) => {
                                    if let Some(data) = tracker.get_data_for_callsign(&callsign) {
                                        if let Some(fp) = &data.fp {
                                            stream.write(build_flightplan_string(&callsign, fp).as_bytes()).ok();
                                        }
                                    }
                                }
                                ClientQueryPayload::IsValidATCQuery(callsign) => {
                                    // Recognize callsign as a valid controller
                                    current_callsign = cq.from.to_string();
                                    // Some ATC clients handle validating ATC differently
                                    if callsign.is_some() {
                                        stream.write(build_validate_atc_string_with_callsign(&current_callsign).as_bytes()).ok();
                                    } else {
                                        stream.write(build_validate_atc_string_without_callsign(&current_callsign).as_bytes()).ok();
                                    }
                                }
                                _ => ()
                            }
                            _ => ()
                        },
                        None => ()
                    }
                }
            }

            // Step stuff
            if let Some(Ok(metar)) = weather.get_next_weather() {
                stream.write(build_metar_string(&current_callsign, &metar).as_bytes()).ok();
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
