use std::time::Instant;

const INTERPOLATE_OFFSET: f32 = 1.0;

fn convert_miles_to_lat(miles: f32) -> f32{
    return miles / 69.0
}

fn convert_miles_to_lon(miles: f32) -> f32{
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

pub struct InterpolatePosition {
    pos: LatLon,
    speed: LatLon,
    time: Instant,
    last_call: LatLon
}

impl Default for InterpolatePosition {
    fn default() -> Self {
        Self {
            time: Instant::now(),
            pos: LatLon {lat: 0.0, lon: 0.0},
            speed: LatLon {lat: 0.0, lon: 0.0},
            last_call: LatLon {lat: 0.0, lon: 0.0},
        }
    }
}

impl InterpolatePosition {
    pub fn new(lat: f32, lon: f32, heading: u32, speed: u32) -> Self {
        let speed = Vector2D::from_heading_and_speed(heading as f32, speed as f32);
        let speed_in_lat_lon = LatLon { // Also to seconds
            lat: convert_miles_to_lat(speed.x) / 3600.0,
            lon: convert_miles_to_lon(speed.y) / 3600.0
        };

        Self {
            pos: LatLon {lat, lon},
            last_call: LatLon {lat, lon},
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