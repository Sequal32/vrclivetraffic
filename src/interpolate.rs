use crate::util::{convert_miles_to_lat, convert_miles_to_lon, LatLon, Vector2D};
use std::time::Instant;

const INTERPOLATE_OFFSET: f32 = 0.0;

pub struct InterpolatePosition {
    pos: LatLon,
    speed: LatLon,
    time: Instant,
    last_call: LatLon,
}

impl Default for InterpolatePosition {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            pos: LatLon { lat: 0.0, lon: 0.0 },
            speed: LatLon { lat: 0.0, lon: 0.0 },
            last_call: LatLon { lat: 0.0, lon: 0.0 },
        }
    }
}

impl InterpolatePosition {
    pub fn new(lat: f32, lon: f32, heading: u32, speed: u32) -> Self {
        let speed = Vector2D::from_heading_and_speed(heading as f32, speed as f32);
        let speed_in_lat_lon = LatLon {
            // Also to seconds
            lat: convert_miles_to_lat(speed.x) / 3600.0,
            lon: convert_miles_to_lon(speed.y) / 3600.0,
        };

        Self {
            pos: LatLon { lat, lon },
            last_call: LatLon { lat, lon },
            speed: speed_in_lat_lon,
            time: Instant::now(),
        }
    }

    pub fn get(&mut self) -> &LatLon {
        let elasped = (self.time.elapsed().as_secs_f32() - INTERPOLATE_OFFSET).max(0.0);
        self.last_call = LatLon {
            lat: self.pos.lat + self.speed.lat * elasped,
            lon: self.pos.lon + self.speed.lon * elasped,
        };
        return &self.last_call;
    }

    pub fn get_no_update(&self) -> &LatLon {
        return &self.last_call;
    }
}
