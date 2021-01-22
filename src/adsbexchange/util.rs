use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};
use super::bincraft::{NavModes, TrackType};

pub fn fill_buf_u32(data_pointer: &[u8], length: usize) -> Box<[u32]> {
    let mut buf = Vec::new();
    let mut rdr = Cursor::new(data_pointer);
    
    for _ in 0..length {
        buf.push(rdr.read_u32::<LittleEndian>().unwrap());
    }

    buf.into_boxed_slice()
}

pub fn fill_buf_u16(data_pointer: &[u8], length: usize) -> Box<[u16]> {
    let mut buf = Vec::new();
    let mut rdr = Cursor::new(data_pointer);
    
    for _ in 0..length {
        buf.push(rdr.read_u16::<LittleEndian>().unwrap());
    }

    buf.into_boxed_slice()
}

pub fn fill_buf_i16(data_pointer: &[u8], length: usize) -> Box<[i16]> {
    let mut buf = Vec::new();
    let mut rdr = Cursor::new(data_pointer);
    
    for _ in 0..length {
        buf.push(rdr.read_i16::<LittleEndian>().unwrap());
    }

    buf.into_boxed_slice()
}

pub fn fill_buf_i32(data_pointer: &[u8], length: usize) -> Box<[i32]> {
    let mut buf = Vec::new();
    let mut rdr = Cursor::new(data_pointer);
    
    for _ in 0..length {
        buf.push(rdr.read_i32::<LittleEndian>().unwrap());
    }

    buf.into_boxed_slice()
}

pub fn convert_char_array_to_string(bytes: &[u8]) -> String {
    let mut result = String::new();

    for i in 0..bytes.len() {
        if bytes[i] == 0 {break}
        result.push(bytes[i] as char);
    }

    return result;
}

pub fn get_track_type_from_num(num: u8) -> TrackType {
    match num {
        0 => TrackType::AdsbIcao,
        1 => TrackType::AdsbIcaoNt,
        2 => TrackType::AdsrIcao,
        3 => TrackType::TisbIcao,
        4 => TrackType::Adsc,
        5 => TrackType::Mlat,
        6 => TrackType::Other,
        7 => TrackType::ModeS,
        8 => TrackType::AdsbOther,
        9 => TrackType::AdsrOther,
        10 => TrackType::TisbTrackfile,
        11 => TrackType::TisbOther,
        12 => TrackType::ModeAc,
        _ => TrackType::Unknown
    }
}

pub fn get_navmodes_from_num(num: u8) -> Vec<NavModes> {
    let mut nav_array = Vec::new();

    if num & 1 != 0 {nav_array.push(NavModes::Autopilot)}
    if num & 2 != 0 {nav_array.push(NavModes::Vnav)}
    if num & 4 != 0 {nav_array.push(NavModes::AltHold)}
    if num & 8 != 0 {nav_array.push(NavModes::Approach)}
    if num & 16 != 0 {nav_array.push(NavModes::Lnav)}
    if num & 32 != 0 {nav_array.push(NavModes::Tcas)}

    return nav_array
}