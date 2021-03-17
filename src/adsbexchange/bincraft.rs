use super::util::{
    convert_char_array_to_string, fill_buf_i16, fill_buf_i32, fill_buf_u16, fill_buf_u32,
    get_navmodes_from_num, get_track_type_from_num,
};
use radix_fmt::radix;

#[derive(Debug)]
pub enum NavModes {
    Autopilot,
    Vnav,
    AltHold,
    Approach,
    Lnav,
    Tcas,
}

#[derive(Debug)]
pub enum TrackType {
    AdsbIcao,
    AdsbIcaoNt,
    AdsrIcao,
    TisbIcao,
    Adsc,
    Mlat,
    Other,
    ModeS,
    AdsbOther,
    AdsrOther,
    TisbTrackfile,
    TisbOther,
    ModeAc,
    Unknown,
}

#[derive(Debug)]
pub struct BoundingLimits {
    pub south: i16,
    pub west: i16,
    pub north: i16,
    pub east: i16,
}

impl BoundingLimits {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let bytes_i16 = fill_buf_i16(bytes, 4);
        Self {
            south: bytes_i16[0],
            west: bytes_i16[1],
            north: bytes_i16[2],
            east: bytes_i16[3],
        }
    }
}

#[derive(Debug)]
pub struct FlightData {
    pub hex: String,
    pub last_pos: Option<u16>,
    pub last_seen: u16,

    pub lat: Option<f64>,
    pub lon: Option<f64>,

    pub alt_baro: Option<f64>,
    pub alt_geom: Option<f64>,
    pub baro_rate: Option<f64>,
    pub geom_rate: Option<f64>,

    pub nav_altitude_mcp: Option<u16>,
    pub nav_altitude_fms: Option<u16>,
    pub nav_qnh: Option<f64>,
    pub nav_heading: Option<f64>,

    pub squawk: Option<String>,
    pub gs: Option<f64>,
    pub mach: Option<f64>,
    pub roll: Option<f64>,

    pub track: Option<f64>,
    pub track_rate: Option<f64>,
    pub mag_heading: Option<f64>,
    pub true_heading: Option<f64>,

    pub wd: Option<i16>,
    pub ws: Option<i16>,
    pub oat: Option<i16>,
    pub tat: Option<i16>,

    pub tas: Option<u16>,
    pub ias: Option<u16>,
    pub rc: u16,
    pub messages: u16,

    pub category: String,
    pub nic: u8,

    pub nav_modes: Option<Vec<NavModes>>,
    pub emergency: Option<u8>,
    pub track_type: TrackType,

    pub airground: u8,
    pub nav_altitude_src: Option<u8>,

    pub sil_type: u8,
    pub adsb_version: u8,

    pub adsr_version: u8,
    pub tisb_version: u8,

    pub nac_p: Option<u8>,
    pub nac_v: Option<u8>,

    pub sil: Option<u8>,
    pub gva: Option<u8>,
    pub sda: Option<u8>,
    pub nic_a: Option<u8>,
    pub nic_c: Option<u8>,

    pub flight: Option<String>,

    pub rssi: f64,
    pub db_flags: u8,
    pub aircraft_type: String,
    pub registration: String,
    pub receiver_count: u8,

    pub nic_baro: Option<u8>,
    pub alert1: Option<u8>,
    pub spi: Option<u8>,
}

impl FlightData {
    pub fn from_bytes(bytes: &[u8], stride: usize) -> Self {
        let s32 = fill_buf_i32(bytes, stride / 4);
        let ua16 = fill_buf_u16(bytes, stride / 2);
        let s16 = fill_buf_i16(bytes, stride / 2);
        let ua8 = &bytes[0..stride];

        // Hex
        let mut hex = format!("{:x}", s32[0] & ((1 << 24) - 1)).to_uppercase();
        if (s32[0] & (1 << 24)) & (1 << 24) != 0 {
            hex = "~".to_owned() + &hex;
        }

        let callsign = convert_char_array_to_string(&ua8[78..86]);
        let aircraft_type = convert_char_array_to_string(&ua8[88..92]);
        let registration = convert_char_array_to_string(&ua8[92..104]);

        let category = radix(ua8[64] as i32, 16).to_string().to_uppercase();
        let squawk = format!("{:0>4}", radix(ua16[16] as i32, 16));

        // Get nav modes
        let nav_modes = if ua8[77] & 4 != 0 {
            Some(get_navmodes_from_num(ua8[66]))
        } else {
            None
        };
        let track_type = get_track_type_from_num((ua8[67] & 240) >> 4);

        Self {
            last_pos: if ua8[73] & 64 != 0 {
                Some(ua16[2] / 10)
            } else {
                None
            },
            last_seen: ua16[3] / 10,
            lat: if ua8[73] & 64 != 0 {
                Some(s32[3] as f64 / 1e6)
            } else {
                None
            },
            lon: if ua8[73] & 64 != 0 {
                Some(s32[2] as f64 / 1e6)
            } else {
                None
            },
            alt_baro: if ua8[73] & 16 != 0 {
                Some(s16[8] as f64 * 25.0)
            } else {
                None
            },
            alt_geom: if ua8[73] & 32 != 0 {
                Some(s16[9] as f64 * 25.0)
            } else {
                None
            },
            baro_rate: if ua8[75] & 1 != 0 {
                Some(s16[10] as f64 * 8.0)
            } else {
                None
            },
            geom_rate: if ua8[75] & 2 != 0 {
                Some(s16[11] as f64 * 8.0)
            } else {
                None
            },
            nav_altitude_mcp: if ua8[76] & 64 != 0 {
                Some(ua16[12] * 4)
            } else {
                None
            },
            nav_altitude_fms: if ua8[76] & 128 != 0 {
                Some(ua16[13] * 4)
            } else {
                None
            },
            nav_qnh: if ua8[76] & 32 != 0 {
                Some(s16[14] as f64 / 10.0)
            } else {
                None
            },
            nav_heading: if ua8[77] & 2 != 0 {
                Some(s16[15] as f64 / 90.0)
            } else {
                None
            },
            squawk: if ua8[76] & 4 != 0 { Some(squawk) } else { None },
            gs: if ua8[73] & 128 != 0 {
                Some(s16[17] as f64 / 10.0)
            } else {
                None
            },
            mach: if ua8[74] & 4 != 0 {
                Some(s16[18] as f64 / 1000.0)
            } else {
                None
            },
            roll: if ua8[74] & 32 != 0 {
                Some(s16[19] as f64 / 100.0)
            } else {
                None
            },
            track: if ua8[74] & 8 != 0 {
                Some(s16[20] as f64 / 90.0)
            } else {
                None
            },
            track_rate: if ua8[74] & 16 != 0 {
                Some(s16[21] as f64 / 100.0)
            } else {
                None
            },
            mag_heading: if ua8[74] & 64 != 0 {
                Some(s16[22] as f64 / 90.0)
            } else {
                None
            },
            true_heading: if ua8[74] & 128 != 0 {
                Some(s16[23] as f64 / 90.0)
            } else {
                None
            },
            wd: if ua8[77] & 16 != 0 {
                Some(s16[24])
            } else {
                None
            },
            ws: if ua8[77] & 16 != 0 {
                Some(s16[25])
            } else {
                None
            },
            oat: if ua8[77] & 32 != 0 {
                Some(s16[26])
            } else {
                None
            },
            tat: if ua8[77] & 32 != 0 {
                Some(s16[27])
            } else {
                None
            },
            tas: if ua8[74] & 2 != 0 {
                Some(ua16[28])
            } else {
                None
            },
            ias: if ua8[74] & 1 != 0 {
                Some(ua16[29])
            } else {
                None
            },
            rc: ua16[30],
            messages: ua16[31],
            category,
            nic: ua8[65],
            nav_modes,
            emergency: if ua8[76] & 8 != 0 {
                Some(ua8[67] & 15)
            } else {
                None
            },
            track_type,
            airground: ua8[68] & 15,
            nav_altitude_src: if ua8[77] & 1 != 0 {
                Some((ua8[68] & 240) >> 4)
            } else {
                None
            },
            sil_type: ua8[69] & 15,
            adsb_version: (ua8[69] & 240) >> 4,
            adsr_version: ua8[70] & 15,
            tisb_version: (ua8[70] & 240) >> 4,
            nac_p: if ua8[75] & 32 != 0 {
                Some(ua8[71] & 15)
            } else {
                None
            },
            nac_v: if ua8[75] & 64 != 0 {
                Some((ua8[71] & 240) >> 4)
            } else {
                None
            },
            sil: if ua8[75] & 128 != 0 {
                Some(ua8[72] & 3)
            } else {
                None
            },
            gva: if ua8[76] & 1 != 0 {
                Some((ua8[72] & 12) >> 2)
            } else {
                None
            },
            sda: if ua8[76] & 2 != 0 {
                Some((ua8[72] & 48) >> 4)
            } else {
                None
            },
            nic_a: if ua8[75] & 4 != 0 {
                Some((ua8[72] & 64) >> 6)
            } else {
                None
            },
            nic_c: if ua8[75] & 8 != 0 {
                Some((ua8[72] & 128) >> 7)
            } else {
                None
            },
            hex,
            flight: if ua8[73] & 8 != 0 {
                Some(callsign.trim().to_owned())
            } else {
                None
            },
            rssi: 10.0 * (ua8[86] as f64 * ua8[86] as f64 / 65025.0 + 1.125e-5).ln() / 10f64.ln(),
            db_flags: ua8[87],
            aircraft_type,
            registration,
            receiver_count: ua8[104],
            nic_baro: if ua8[75] & 16 != 0 {
                Some(ua8[73] & 1)
            } else {
                None
            },
            alert1: if ua8[77] & 8 != 0 {
                Some(ua8[73] & 2)
            } else {
                None
            },
            spi: if ua8[76] & 16 != 0 {
                Some(ua8[73] & 4)
            } else {
                None
            },
        }
    }
}

#[derive(Debug)]
pub struct BinCraftData {
    pub time: f64,
    pub ac_count: u32,
    pub global_index: u32,
    pub limits: BoundingLimits,
    pub aircraft: Vec<FlightData>,
}

impl BinCraftData {
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let vals: Box<[u32]> = fill_buf_u32(bytes, 5);

        let time = (vals[0] / 1000) as f64 + (vals[1] as f64 * 4294867.296);
        let stride = vals[2] as usize;
        let ac_count = vals[3];
        let global_index = vals[4];

        let limits = BoundingLimits::from_bytes(&bytes[20..]);

        // Aircraft list
        let mut aircraft = Vec::new();
        let mut offset = stride;

        while offset < bytes.len() {
            aircraft.push(FlightData::from_bytes(&bytes[offset..], stride));

            offset += stride;
        }

        Self {
            time,
            ac_count,
            global_index,
            limits,
            aircraft,
        }
    }
}
