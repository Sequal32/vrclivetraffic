use std::sync::{Arc, Mutex};

use reqwest::blocking::Client;
use regex;
use serde_json::{Value, Map};

use crate::request::Request;

const ENDPOINT: &str = "https://flightaware.com/live/flight/";

#[derive(Debug)]
pub struct FlightPlan {
    pub origin: String,
    pub destination: String,
    pub equipment: String,
    pub speed: u64,
    pub altitude: u64,
    pub route: String,

}

fn get_origin(flight_data: &Map<String, Value>) -> Option<String> {
    Some(flight_data.get("origin")?.as_object()?.get("icao")?.as_str().unwrap_or_default().to_string())
}

fn get_destination(flight_data: &Map<String, Value>) -> Option<String> {
    Some(flight_data.get("destination")?.as_object()?.get("icao")?.as_str().unwrap_or_default().to_string())
}

fn get_equipment(flight_data: &Map<String, Value>) -> Option<String> {
    Some(flight_data.get("aircraft")?.as_object()?.get("type")?.as_str().unwrap_or_default().to_string())
}

fn get_flightplan_from_json(data: &Value) -> Option<FlightPlan> {
    let flights = data.as_object()?.get("flights")?.as_object()?;
    let (_, first_flight) = flights.iter().next()?;
    
    let flight_data = first_flight.as_object()?;
    let flight_plan = flight_data.get("flightPlan")?.as_object()?;

    let origin = get_destination(flight_data).unwrap_or_default();
    let destination = get_origin(flight_data).unwrap_or_default();
    let equipment = get_equipment(flight_data).unwrap_or_default();

    
    let speed = flight_plan.get("speed")?.as_u64().unwrap_or(0);
    let mut altitude = flight_plan.get("altitude")?.as_u64().unwrap_or(0);
    let route = flight_plan.get("route")?.as_str().unwrap_or("").to_string();

    if altitude < 1000 {altitude = altitude * 100}

    return Some(FlightPlan {
        origin,
        equipment,
        destination,
        speed,
        altitude,
        route
    });
}

pub struct FlightPlanResult {
    pub id: String,
    pub callsign: String,
    pub fp: FlightPlan
}

pub struct FlightAware {
    client: Arc<Mutex<Client>>,
    flightplan_request: Request<Result<FlightPlanResult, FlightAwareError>>
}

#[derive(Debug)]
pub enum FlightAwareError {
    RequestFailed(String),
    ParseError(String),
}

impl FlightAware {
    pub fn new() -> Self {
        Self {
            client: Arc::new(Mutex::new(Client::new())),
            flightplan_request: Request::new()
        }
    }

    pub fn request_flightplan(&self, id: &str, callsign: &str) {
        let exp = regex::Regex::new(r"var trackpollBootstrap = (\{.+\});").unwrap();
        
        let tx = self.flightplan_request.get_handle();
        let callsign = callsign.to_string();
        let id = id.to_string();
        let client = self.client.clone();
        std::thread::spawn(move || {
            let response = client.lock().unwrap().get(format!("{}{}", ENDPOINT, callsign).as_str()).send();
            if !response.is_ok() {
                tx.send(Err(FlightAwareError::RequestFailed(callsign))).ok();
                return
            }
    
            if let Ok(text) = response.unwrap().text() {
                let mut data: &str = "";
    
                for cap in exp.captures(text.as_str()) {
                    data = cap.get(1).unwrap().as_str();
                    break;
                }
                //
                if let Ok(data) = serde_json::from_str(data) {
                    let data: Value = data;
                    match get_flightplan_from_json(&data) {
                        Some(data) => {
                            tx.send(Ok(FlightPlanResult {callsign, id: id, fp: data})).ok();
                            return
                        },
                        None => {
                            tx.send(Err(FlightAwareError::ParseError(callsign))).ok();
                            return
                        }
                    }
                }
            }
    
            tx.send(Err(FlightAwareError::ParseError(callsign))).ok();
        });
    }

    pub fn get_next_flightplan(&self) -> Option<Result<FlightPlanResult, FlightAwareError>> {
        return self.flightplan_request.get_next();
    }
}