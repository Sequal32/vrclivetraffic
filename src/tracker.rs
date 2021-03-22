use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::time::Instant;

use log::{info, warn};

use crate::airports::Airports;
use crate::flightaware::{FlightAware, FlightPlan};
use crate::flightradar::FlightRadar;
use crate::interpolate::InterpolatePosition;
use crate::util::{AircraftProvider, Bounds};
use crate::{adsbexchange::AdsbExchange, util::BoxedData};

const POLL_RATE: u64 = 3;

pub struct Tracker {
    providers: Vec<Box<dyn AircraftProvider>>,
    faware: FlightAware,

    buffer: VecDeque<Vec<(String, BoxedData)>>,
    is_buffering: bool,
    tracking: HashMap<String, TrackData>,
    callsign_map: HashMap<String, String>,
    time: Instant,

    floor: i32,
    ceiling: i32,
}

impl Tracker {
    pub fn new(radar_loc: &Bounds, airports: Rc<Airports>, floor: i32, ceiling: i32) -> Self {
        Self {
            providers: vec![
                Box::new(FlightRadar::new(radar_loc, airports)),
                Box::new(AdsbExchange::new(radar_loc)),
            ],
            faware: FlightAware::new(),

            buffer: VecDeque::new(),
            is_buffering: false,
            tracking: HashMap::new(),
            callsign_map: HashMap::new(),
            time: Instant::now(),

            floor,
            ceiling,
        }
    }

    pub fn run_faware(&self) {
        self.faware.run();
    }

    fn try_update_flightplan(&mut self, id: &String) {
        let data = match self.tracking.get_mut(id) {
            Some(d) => d,
            None => return,
        };
        // Already updated flight plan
        if data.fp.is_some() || data.fp_did_try_update {
            return;
        }
        // FP on request
        data.fp_did_try_update = true;

        // Only update airliners
        if data.ac_data.is_airline() {
            let callsign = data.ac_data.callsign();

            info!("Requesting flight plan for {}", callsign);

            self.faware.request_flightplan(id, callsign);
        }
    }

    fn update_flightplan(&mut self, id: &String, fp: FlightPlan) {
        if let Some(track_data) = self.tracking.get_mut(id) {
            track_data.fp = Some(fp);
        }
    }

    /// Returns the original data if not passed in in an Option
    fn check_and_create_new_aircraft(&mut self, id: &String, data: BoxedData) -> Option<BoxedData> {
        // if aircraft was created
        if self.tracking.get(id).is_none() {
            // Callsign doesn't already exist
            if self.callsign_map.contains_key(data.callsign()) {
                return Some(data);
            }
            // Callsign is invalid (FlightRadar24 sometimes puts the aircraft type as the callsign...)
            if data.callsign().len() <= 4 {
                return Some(data);
            }

            info!("Creating {}", data.callsign());

            self.callsign_map
                .insert(data.callsign().to_string(), id.clone());

            self.tracking
                .insert(id.clone(), TrackData::new(id.clone(), data));

            return None;
        }
        return Some(data);
    }

    // Removes aircraft that have been lost on radar
    fn remove_untracked(&mut self, keys: &HashSet<String>) {
        let mut to_remove = Vec::new();

        for key in self.tracking.keys() {
            if !keys.contains(key) {
                to_remove.push(key.clone());
            }
        }

        for removing in to_remove {
            let data = self.tracking.remove(&removing).unwrap();
            info!("Removing {}", data.ac_data.callsign());
            self.callsign_map.remove(data.ac_data.callsign());
        }
    }

    fn get_next_aircraft_update(&mut self) -> Option<Vec<(String, BoxedData)>> {
        // A vector is used here instead as we want to keep duplicates in case one of the provider's data fails to validate
        let mut all_data = Vec::new();

        for provider in self.providers.iter_mut() {
            match provider.get_aircraft() {
                Ok(aircraft) => {
                    for (key, value) in aircraft {
                        all_data.push((key, value));
                    }
                }
                Err(e) => warn!(
                    "Error fetching data from {}! Reason: {:?}",
                    provider.get_name(),
                    e,
                ),
            }
        }

        self.buffer.push_back(all_data);

        if !self.is_buffering {
            return self.buffer.pop_front();
        } else {
            return None;
        }
    }

    fn update_position(&mut self, id: &str, new_ac_data: &BoxedData) {
        let current_data = match self.tracking.get_mut(id) {
            Some(d) => d,
            None => return,
        };

        if new_ac_data.timestamp() <= current_data.last_position_update {
            return;
        }

        current_data.position = InterpolatePosition::new(
            new_ac_data.latitude(),
            new_ac_data.longitude(),
            new_ac_data.heading(),
            new_ac_data.ground_speed(),
        );
        current_data.last_position_update = new_ac_data.timestamp();
        current_data.at_last_position_update = Instant::now();
    }

    fn update_aircraft(&mut self) {
        let aircraft = match self.get_next_aircraft_update() {
            Some(a) => a,
            None => return,
        };

        let mut processed_ids = HashSet::new();

        for (id, aircraft) in aircraft {
            // Providers may be tracking the same aircraft
            if processed_ids.contains(&id) {
                continue;
            }
            // Invalid callsigns/callsign not received
            if aircraft.callsign().trim() == "" {
                continue;
            }
            if aircraft.altitude() < self.floor || aircraft.altitude() > self.ceiling {
                continue;
            }

            if let Some(new_data) = self.check_and_create_new_aircraft(&id, aircraft) {
                self.update_position(&id, &new_data);
                self.try_update_flightplan(&id);
            }

            processed_ids.insert(id);
        }

        self.remove_untracked(&processed_ids);
    }

    // Step
    fn step_flightplan(&mut self) {
        if let Some(result) = self.faware.get_next_flightplan() {
            match result {
                Ok(fp) => {
                    self.update_flightplan(&fp.id, fp.fp);
                    info!("Received flight plan for {}", fp.callsign);
                }
                Err(e) => info!("Could not receive flight plan because {:?}", e),
            }
        }
    }

    // Interpolate this
    pub fn aircraft_exists(&self, id: &String) -> bool {
        return self.tracking.contains_key(id);
    }

    pub fn get_aircraft_data(&mut self) -> Vec<&mut TrackData> {
        return self.tracking.values_mut().map(|x| x).collect();
    }

    pub fn get_data_for_callsign(&self, callsign: &String) -> Option<&TrackData> {
        return Some(self.tracking.get(self.callsign_map.get(callsign)?)?);
    }

    pub fn step(&mut self) {
        let elasped = self.time.elapsed().as_secs();
        if elasped > POLL_RATE {
            self.update_aircraft();
            self.time = Instant::now();
        }

        self.step_flightplan();
    }

    pub fn start_buffering(&mut self) {
        self.is_buffering = true;
    }

    pub fn stop_buffering(&mut self) {
        self.is_buffering = false;
    }

    pub fn is_buffering(&self) -> bool {
        self.is_buffering
    }
}

pub struct TrackData {
    pub id: String,
    // Flight Plan
    pub fp_did_try_update: bool,
    pub fp: Option<FlightPlan>,
    // Position
    last_position_update: u64,
    pub at_last_position_update: Instant,
    pub position: InterpolatePosition,
    // Meta data
    pub ac_data: BoxedData,
}

impl TrackData {
    pub fn new(id: String, ac_data: BoxedData) -> Self {
        Self {
            ac_data,
            id,
            fp: None,
            fp_did_try_update: false,
            last_position_update: 0,
            at_last_position_update: Instant::now(),
            position: InterpolatePosition::default(),
        }
    }
}
