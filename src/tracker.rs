use std::{collections::HashMap, time::Instant, collections::{HashSet, VecDeque}};

use crate::{adsbexchange::AdsbExchange, util::{AircraftProvider, Bounds, MinimalAircraftData}};
use crate::{flightradar::{ FlightRadar}, interpolate::InterpolatePosition};
use crate::flightaware::{FlightPlan, FlightAware};

const POLL_RATE: u64 = 3;

pub struct Tracker {
    providers: Vec<Box<dyn AircraftProvider>>,
    faware: FlightAware,

    buffer: VecDeque<Vec<(String, MinimalAircraftData)>>,
    is_buffering: bool,
    tracking: HashMap<String, TrackData>,
    callsign_map: HashMap<String, String>,
    time: Instant,

    floor: i32,
    ceiling: i32
}

impl Tracker {
    pub fn new(radar_loc: &Bounds, floor: i32, ceiling: i32) -> Self {
        Self {
            providers: vec![
                Box::new(FlightRadar::new(radar_loc)),
                Box::new(AdsbExchange::new(radar_loc))
            ],
            faware: FlightAware::new(),

            buffer: VecDeque::new(),
            is_buffering: false,
            tracking: HashMap::new(),
            callsign_map: HashMap::new(),
            time: Instant::now(),

            floor,
            ceiling
        }
    }

    pub fn run(&self) {
        self.faware.run();
    }

    fn try_update_flightplan(&mut self, id: &String) {
        let data = self.tracking.get_mut(id);
        if data.is_none() {return}

        let data = data.unwrap();
        if data.fp.is_some() || data.fp_did_try_update {return}

        data.fp_did_try_update = true;
        self.faware.request_flightplan(id, &data.data.callsign);
    }

    fn update_flightplan(&mut self, id: &String, fp: FlightPlan) {
        if let Some(track_data) = self.tracking.get_mut(id) {
            track_data.fp = Some(fp);
        }
        
    }

    fn update_position(&mut self, id: &String, new_data: &MinimalAircraftData) {
        let prev_data = self.tracking.get_mut(id);
        if prev_data.is_none() {return}
        let prev_data = prev_data.unwrap();

        if new_data.timestamp <= prev_data.last_position_update {return}

        prev_data.data = new_data.clone();
        prev_data.position = InterpolatePosition::new(new_data.latitude, new_data.longitude, new_data.heading, new_data.ground_speed);
        prev_data.last_position_update = new_data.timestamp;
        prev_data.at_last_position_update = Some(Instant::now());
    }

    fn check_and_create_new_aircraft(&mut self, id: &String, data: &MinimalAircraftData) -> bool { // if aircraft was created        
        if self.tracking.get(id).is_none() {
            // Callsign is valid (FlightRadar24 sometimes puts the aircraft type as the callsign...)
            if data.callsign.len() <= 4 {return false}
            // Callsign doesn't already exist
            if self.callsign_map.contains_key(&data.callsign) {return false}
            self.callsign_map.insert(data.callsign.clone(), id.clone());

            self.tracking.insert(id.clone(), TrackData {
                data: data.clone(),
                id: id.clone(),
                .. Default::default()
            });
        }
        return true;
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
            self.callsign_map.remove(&data.data.callsign);
        }
    }

    fn get_next_aircraft_update(&mut self) -> Option<Vec<(String, MinimalAircraftData)>> {
        // A vector is used here instead as we want to keep duplicates in case one of the provider's data fails to validate
        let mut all_data = Vec::new();

        for provider in self.providers.iter_mut() {
            match provider.get_aircraft() {
                Ok(aircraft) => {
                    for (key, value) in aircraft {
                        all_data.push((key, value));
                    }
                }
                Err(e) => println!("Error fetching data from {}! Reason: {:?}", provider.get_name(), e)
            }
        }

        self.buffer.push_back(all_data);

        if !self.is_buffering {
            return self.buffer.pop_front();
        } else {
            return None;
        }
    }

    fn update_aircraft(&mut self) {
        let aircraft = match self.get_next_aircraft_update() {
            Some(a) => a,
            None => return
        };

        let mut processed_ids = HashSet::new();

        for (id, aircraft) in aircraft {
            // Providers may be tracking the same aircraft
            if processed_ids.contains(&id) {continue}
            // Invalid callsigns/callsign not received
            if aircraft.callsign.trim() == "" {continue}
            if aircraft.altitude < self.floor || aircraft.altitude > self.ceiling {continue}
            if !self.check_and_create_new_aircraft(&id, &aircraft) {continue} // can't use aircrat
            

            self.update_position(&id, &aircraft);
            self.try_update_flightplan(&id);

            processed_ids.insert(id);
        }

        self.remove_untracked(&processed_ids);
    }

    // Step
    fn step_flightplan(&mut self) {
        if let Some(result) = self.faware.get_next_flightplan() {
            match result {
                Ok(fp) => self.update_flightplan(&fp.id, fp.fp),
                Err(_) => ()
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
        return Some(self.tracking.get(self.callsign_map.get(callsign)?)?)
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

#[derive(Default)]
pub struct TrackData {
    pub id: String,
    // Flight Plan
    pub fp_did_try_update: bool,
    pub fp: Option<FlightPlan>,
    // Position
    last_position_update: u64,
    pub at_last_position_update: Option<Instant>,
    pub position: InterpolatePosition,
    // Meta data
    pub data: MinimalAircraftData
}