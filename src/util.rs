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