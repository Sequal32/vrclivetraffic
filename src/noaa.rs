use std::{sync::{Mutex, Arc}, io::BufRead, io::BufReader};

use blocking::Client;
use reqwest::blocking;
use csv;

use crate::request::Request;

const METAR_ENDPOINT: &str = "https://www.aviationweather.gov/adds/dataserver_current/httpparam?dataSource=metars&requestType=retrieve&format=csv&hoursBeforeNow=2&mostRecent=true&stationString=";

#[derive(Debug)]
pub enum MetarError {
    RequestFailed,
    ParseError
}

pub struct NoaaWeather {
    weather_request: Request<Result<String, MetarError>, String>,
    client: Arc<Mutex<Client>>
}

impl NoaaWeather {
    pub fn new() -> Self {
        Self {
            weather_request: Request::new(),
            client: Arc::new(Mutex::new(Client::new()))
        }
    }

    pub fn run(&self) {
        let client = self.client.clone();

        self.weather_request.run(move |icao| {
            // Get data from NOAA
            let response = match client.lock().unwrap().get(format!("{}{}", METAR_ENDPOINT, icao).as_str()).send() {
                Ok(r) => r,
                Err(_) => return Err(MetarError::RequestFailed)
            };

            let text = match response.text() {
                Ok(t) => t,
                Err(_) => return Err(MetarError::RequestFailed)
            };
            // Ignore first lines of metadata
            let text: String = text.split("\n").skip(5).map(|x| x.to_string()).collect::<Vec<String>>().join("\n");

            // Parse csv
            let mut reader = csv::Reader::from_reader(text.as_bytes());
            for record in reader.records() {
                match record {
                    Ok(record) => {
                        return Ok(record[0].to_string())
                    }
                    _ => ()
                }
            };
            return Err(MetarError::ParseError);
        });
    }

    pub fn request_weather(&self, icao: &str) {
        self.weather_request.give_job(icao.to_string());
    }

    pub fn get_next_weather(&self) -> Option<Result<String, MetarError>> {
        return self.weather_request.get_next();
    }
}