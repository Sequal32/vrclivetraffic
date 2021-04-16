use attohttpc;
use chrono::{serde::ts_seconds, DateTime, Utc};
use regex;
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::Value;

use crate::error::Error;
use crate::request::Request;

const ENDPOINT: &str = "https://flightaware.com/live/flight/";

#[derive(Debug)]
pub struct FlightPlan {
    pub origin: Airport,
    pub destination: Airport,
    pub equipment: Aircraft,
    pub fp: PartialFlightPlan,
    pub arrival_time: Option<Times>,
    pub departure_time: Option<Times>,
}

#[derive(Debug, Deserialize, Default)]
pub struct Aircraft {
    #[serde(rename = "type")]
    #[serde(default)]
    pub ac_type: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct PartialFlightPlan {
    #[serde(default)]
    pub speed: u64,
    #[serde(default)]
    pub altitude: u64,
    #[serde(default)]
    pub route: String,
}

#[derive(Debug, Deserialize, Default)]
pub struct Airport {
    #[serde(default)]
    pub icao: String,
    pub gate: Option<String>,
    pub terminal: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Times {
    #[serde(with = "ts_seconds")]
    pub scheduled: DateTime<Utc>,
}

fn deserialize_or_none<T>(data: &Value, key: &str) -> Option<T>
where
    T: DeserializeOwned,
{
    serde_json::from_value(data.get(key)?.to_owned()).ok()
}

fn deserialize_or_default<T>(data: &Value, key: &str) -> T
where
    T: Default + DeserializeOwned,
{
    deserialize_or_none(data, key).unwrap_or_default()
}

fn get_flightplan_from_json(data: &Value) -> Option<FlightPlan> {
    let flights = data.as_object()?.get("flights")?.as_object()?;
    let (_, first_flight) = flights.into_iter().next()?;

    let origin: Airport = deserialize_or_none(first_flight, "origin")?; // No origin means there is pretty much no other data available
    let destination: Airport = deserialize_or_default(first_flight, "destination");
    let aircraft: Aircraft = deserialize_or_default(first_flight, "aircraft");
    let arrival_time: Option<Times> = deserialize_or_none(first_flight, "gateArrivalTimes");
    let departure_time: Option<Times> = deserialize_or_none(first_flight, "gateDepartureTimes");

    let mut partial: PartialFlightPlan = deserialize_or_default(first_flight, "flightPlan");

    partial.altitude = if partial.altitude < 1000 {
        partial.altitude * 100
    } else {
        partial.altitude
    };

    return Some(FlightPlan {
        origin,
        destination,
        fp: partial,
        equipment: aircraft,
        arrival_time,
        departure_time,
    });
}

#[derive(Debug)]
struct FlightPlanRequest {
    id: String,
    callsign: String,
}

#[derive(Debug)]
pub struct FlightPlanResult {
    pub id: String,
    pub callsign: String,
    pub fp: FlightPlan,
}

pub struct FlightAware {
    flightplans: Request<Result<FlightPlanResult, Error>, FlightPlanRequest>,
    pub running: bool,
}

impl FlightAware {
    pub fn new() -> Self {
        Self {
            flightplans: Request::new(5),
            running: false,
        }
    }

    pub fn run(&mut self) {
        let exp = regex::Regex::new(r"var trackpollBootstrap = (\{.+\});").unwrap();
        self.running = true;

        self.flightplans.run(move |job| {
            // Get data from flightaware
            let text = attohttpc::get(ENDPOINT.to_owned() + &job.callsign)
                .send()?
                .error_for_status()?
                .text()?;

            let mut data: &str = "";
            // Parse json from html
            for cap in exp.captures(&text) {
                data = cap.get(1).unwrap().as_str();
                break;
            }

            // Deserialize into Value
            let data: Value = serde_json::from_str(data)?;

            match get_flightplan_from_json(&data) {
                Some(fp) => Ok(FlightPlanResult {
                    callsign: job.callsign,
                    id: job.id,
                    fp: fp,
                }),
                None => Err(Error::NotFound),
            }
        });
    }

    pub fn request_flightplan(&self, id: &str, callsign: &str) {
        self.flightplans.give_job(FlightPlanRequest {
            id: id.to_string(),
            callsign: callsign.to_string(),
        });
    }

    pub fn get_next_flightplan(&self) -> Option<Result<FlightPlanResult, Error>> {
        return self.flightplans.get_next();
    }
}
