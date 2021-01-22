use std::collections::HashMap;

use crate::error::Error;

pub fn convert_miles_to_lat(miles: f32) -> f32{
    return miles / 69.0
}

pub fn convert_miles_to_lon(miles: f32) -> f32{
    return miles / 54.6
}

#[derive(Debug, Default, Clone)]
pub struct LatLon {
    pub lat: f32,
    pub lon: f32
}

#[derive(Debug, Default, Clone)]
pub struct Vector2D {
    pub x: f32,
    pub y: f32
}

impl Vector2D {
    pub fn from_heading_and_speed(heading: f32, speed: f32) -> Self {
        // Split speed into componenets
        // let angle = heading + 180 % 360;

        Self {
            x: heading.to_radians().cos() * speed,
            y: heading.to_radians().sin() * speed
        }
    }
}

#[derive(Debug, Default, Clone)]
pub struct Bounds {
    pub lat1: f32,
    pub lon1: f32,
    pub lat2: f32,
    pub lon2: f32,
}

#[derive(Debug, Default, Clone)]
pub struct MinimalAircraftData {
    pub squawk: String,
    pub callsign: String,
    pub is_on_ground: bool,
    pub latitude: f32,
    pub longitude: f32,
    pub heading: u32,
    pub ground_speed: u32,
    pub timestamp: u64,
    pub altitude: i32,
}

pub trait AircraftProvider {
    fn get_aircraft(&mut self) -> Result<HashMap<String, MinimalAircraftData>, Error>;
    fn get_name(&self) -> &str;
}