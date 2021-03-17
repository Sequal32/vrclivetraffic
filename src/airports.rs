use crate::util::{convert_miles_to_lat, convert_miles_to_lon, Bounds, LatLon};
use csv;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug)]
pub struct AirportData {
    pub id: u32,
    pub name: String,
    pub city: String,
    pub country: String,
    pub iata: String,
    pub icao: String,
    pub lat: f32,
    pub lon: f32,
    pub elevation: i32,
    pub utc: f32,
    pub timezone: String,
    filler: String,
    filler2: String,
    pub database: String,
}

pub struct Airports {
    db: HashMap<String, AirportData>,
}

impl Airports {
    pub fn new(filename: &str) -> Result<Self, csv::Error> {
        let mut reader = csv::Reader::from_path(filename)?;
        let mut db = HashMap::new();

        for record in reader.deserialize() {
            let record: AirportData = record?;
            db.insert(record.icao.clone(), record);
        }

        Ok(Self { db })
    }

    pub fn get_lat_lon(&self, icao: &String) -> Option<LatLon> {
        let data = self.db.get(icao)?;
        Some(LatLon {
            lat: data.lat,
            lon: data.lon,
        })
    }

    pub fn get_bounds_from_radius(&self, icao: &String, radius: f32) -> Option<Bounds> {
        let center = self.get_lat_lon(icao)?;
        let offset = LatLon {
            lat: convert_miles_to_lat(radius),
            lon: convert_miles_to_lon(radius),
        };

        Some(Bounds {
            lat1: center.lat + offset.lat, // Right
            lon1: center.lon - offset.lon, // Bottom
            lat2: center.lat - offset.lat, // Left
            lon2: center.lon + offset.lon, // Top
        })
    }
}
