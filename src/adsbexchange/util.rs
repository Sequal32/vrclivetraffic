use super::bincraft::{NavModes, TrackType};

pub fn as_other_array<T>(bytes: &[u8], len: usize) -> &[T] {
    unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const T, len) }
}

pub fn convert_char_array_to_string(bytes: &[u8]) -> String {
    return String::from_utf8_lossy(bytes).to_string();
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
        _ => TrackType::Unknown,
    }
}

pub fn get_navmodes_from_num(num: u8) -> Vec<NavModes> {
    let mut nav_array = Vec::new();

    if num & 1 != 0 {
        nav_array.push(NavModes::Autopilot)
    }
    if num & 2 != 0 {
        nav_array.push(NavModes::Vnav)
    }
    if num & 4 != 0 {
        nav_array.push(NavModes::AltHold)
    }
    if num & 8 != 0 {
        nav_array.push(NavModes::Approach)
    }
    if num & 16 != 0 {
        nav_array.push(NavModes::Lnav)
    }
    if num & 32 != 0 {
        nav_array.push(NavModes::Tcas)
    }

    return nav_array;
}
