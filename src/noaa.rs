use attohttpc;
use csv;

use crate::error::Error;
use crate::request::Request;

const METAR_ENDPOINT: &str = "https://www.aviationweather.gov/adds/dataserver_current/httpparam?dataSource=metars&requestType=retrieve&format=csv&hoursBeforeNow=2&mostRecent=true&stationString=";

pub struct NoaaWeather {
    weather_request: Request<Result<String, Error>, String>,
}

impl NoaaWeather {
    pub fn new() -> Self {
        Self {
            weather_request: Request::new(1),
        }
    }

    pub fn run(&self) {
        self.weather_request.run(move |icao| {
            // Get data from NOAA
            let text = attohttpc::get(METAR_ENDPOINT.to_owned() + &icao)
                .send()?
                .error_for_status()?
                .text()?;
            // Ignore first lines of metadata
            let text  = text.split("\n").skip(5).map(|x| x.to_string()).collect::<Vec<String>>().join("\n");

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
            return Err(Error::NotFound);
        });
    }

    pub fn request_weather(&self, icao: &str) {
        self.weather_request.give_job(icao.to_string());
    }

    pub fn get_next_weather(&self) -> Option<Result<String, Error>> {
        return self.weather_request.get_next();
    }
}