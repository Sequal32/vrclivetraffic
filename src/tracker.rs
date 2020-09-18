use std::{collections::HashMap, time::Instant, collections::{HashSet, VecDeque}, collections::hash_map::Keys};

use crate::util::Bounds;
use crate::{flightradar::{AircraftData, FlightRadar}, interpolate::InterpolatePosition};
use crate::flightaware::{FlightPlan, FlightAware};

const POLL_RATE: u64 = 3;

pub struct Tracker {
    radar: FlightRadar,
    faware: FlightAware,

    buffer: VecDeque<HashMap<String, AircraftData>>,
    is_buffering: bool,
    tracking: HashMap<String, TrackData>,
    callsign_set: HashSet<String>,
    time: Instant,

    floor: i32,
    ceiling: i32
}

impl Tracker {
    pub fn new(radar_loc: &Bounds, floor: i32, ceiling: i32) -> Self {
        Self {
            radar: FlightRadar::new(radar_loc),
            faware: FlightAware::new(),

            buffer: VecDeque::new(),
            is_buffering: false,
            tracking: HashMap::new(),
            callsign_set: HashSet::new(),
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

    fn update_position(&mut self, id: &String, new_data: &AircraftData) {
        let prev_data = self.tracking.get_mut(id);
        if prev_data.is_none() {return}
        let prev_data = prev_data.unwrap();

        if new_data.timestamp <= prev_data.last_position_update {return}

        prev_data.data = new_data.clone();
        prev_data.position = InterpolatePosition::new(new_data.latitude, new_data.longitude, new_data.bearing, new_data.speed);
        prev_data.last_position_update = new_data.timestamp;
        prev_data.at_last_position_update = Some(Instant::now());
    }

    fn check_and_create_new_aircraft(&mut self, id: &String, data: &AircraftData) -> bool { // if aircraft was created        
        if self.tracking.get(id).is_none() {
            if self.callsign_set.contains(&data.callsign) {return false}
            self.callsign_set.insert(data.callsign.clone());
            self.tracking.insert(id.clone(), TrackData {
                data: data.clone(),
                id: id.clone(),
                .. Default::default()
            });
        }
        return true;
    }

    fn remove_untracked(&mut self, keys: Keys<String, AircraftData>) {
        let new_data: HashSet<String> = keys.map(|x| x.clone()).collect();
        let old_data: HashSet<String> = self.tracking.keys().map(|x| x.clone()).collect();
        for untracked in old_data.difference(&new_data) {
            let data = self.tracking.remove(untracked).unwrap();
            self.callsign_set.remove(&data.data.callsign);
        }
    }

    fn get_next_aircraft_update(&mut self) -> Option<HashMap<String, AircraftData>> {
        if let Ok(aircraft) = self.radar.get_aircraft() {
            self.buffer.push_back(aircraft);
        }

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

        for (id, aircraft) in aircraft.iter() {
            // Invalid callsigns/callsign not received
            if aircraft.callsign.trim() == "" {continue}
            if aircraft.altitude < self.floor || aircraft.altitude > self.ceiling {continue}
            if !self.check_and_create_new_aircraft(id, aircraft) {continue} // can't use aircrat

            self.update_position(id, aircraft);
            self.try_update_flightplan(id);
        }

        self.remove_untracked(aircraft.keys());
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
    pub data: AircraftData
}