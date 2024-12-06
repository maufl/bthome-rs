use std::io::{Cursor, Read};

pub const BTHOME_UUID16: u16 = 0xFCD2;
pub const BTHOME_UUID: u128 = 0x0000FCD2_0000_1000_8000_00805F9B34FB;


#[derive(Debug)]
pub enum Error {
    IoError(std::io::Error),
    InvalidTextEncoding,
    InvalidObjectId(u8),
    InvalidButtonEvent(u8),
    InvalidDimmerEvent(u8),
}

#[repr(C)]
#[derive(Debug)]
pub enum ButtonEvent {
    None = 0x00,
    Press = 0x01,
    DoublePress = 0x02,
    TriplePress = 0x03,
    LongPress = 0x04,
    LongDoublePress = 0x05,
    LongTriplePress = 0x06,
    HoldPress = 0x80,
}

impl std::convert::TryFrom<u8> for ButtonEvent {
    type Error = Error;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == ButtonEvent::None as u8 => Ok(ButtonEvent::None),
            x if x == ButtonEvent::Press as u8 => Ok(ButtonEvent::Press),
            x if x == ButtonEvent::DoublePress as u8 => Ok(ButtonEvent::DoublePress),
            x if x == ButtonEvent::TriplePress as u8 => Ok(ButtonEvent::TriplePress),
            x if x == ButtonEvent::LongPress as u8 => Ok(ButtonEvent::LongPress),
            x if x == ButtonEvent::LongDoublePress as u8 => Ok(ButtonEvent::LongDoublePress),
            x if x == ButtonEvent::LongTriplePress as u8 => Ok(ButtonEvent::LongTriplePress),
            x if x == ButtonEvent::HoldPress as u8 => Ok(ButtonEvent::HoldPress),
            _ => Err(Error::InvalidButtonEvent(v)),
        }
    }
}

#[repr(C)]
#[derive(Debug)]
pub enum DimmerEvent {
    None = 0x00,
    RotateLeft = 0x01,
    RotateRight = 0x02,
}

impl std::convert::TryFrom<u8> for DimmerEvent {
    type Error = Error;
    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == DimmerEvent::None as u8 => Ok(DimmerEvent::None),
            x if x == DimmerEvent::RotateLeft as u8 => Ok(DimmerEvent::RotateLeft),
            x if x == DimmerEvent::RotateRight as u8 => Ok(DimmerEvent::RotateRight),
            _ => Err(Error::InvalidObjectId(v)),
        }
    }
}

macro_rules! value_parsers {
    ($(($bttype:ident, $rtype:ident, $rsize:literal$(, $btsize:literal)?),)*) => {

        #[allow(dead_code)]
        mod float_from {
            use crate::{Read, ObjectValue, Error};
            $(pub(crate) fn $bttype(data: &mut impl Read, factor: f32) -> Result<ObjectValue, Error> {
                let mut bytes = [0u8; $rsize];
                data.read_exact(&mut bytes$([..$btsize])?)?;
                Ok(ObjectValue::Float($rtype::from_le_bytes(bytes) as f32 * factor))
            })*
        }
        
        #[allow(dead_code)]
        mod int_from {
            use crate::{Read, ObjectValue, Error};
            $(pub(crate) fn $bttype(data: &mut impl Read) -> Result<ObjectValue, Error> {
                let mut bytes = [0u8; $rsize];
                data.read_exact(&mut bytes$([..$btsize])?)?;
                Ok(ObjectValue::Int($rtype::from_le_bytes(bytes) as i64))
            })*
        }
    };
}

value_parsers! {
    (uint8, u8, 1),
    (sint8, i8, 1),
    (uint16, u16, 2),
    (sint16, i16, 2),
    (uint24, u32, 4, 3),
    (sint24, i32, 4, 3),
    (uint32, u32, 4),
    (sint32, i32, 4),
    (uint48, u64, 8, 6),
    (uint64, u64, 8, 6),
}

fn read_bool(data: &mut impl Read) -> Result<ObjectValue, Error> {
    let mut bytes = [0u8; 1];
    data.read_exact(&mut bytes)?;
    Ok(ObjectValue::Bool(u8::from_le_bytes(bytes) == 0u8))
}

fn read_bytes(data: &mut impl Read) -> Result<ObjectValue, Error> {
    let mut size = [0u8; 1];
    data.read_exact(&mut size)?;
    let mut bytes = vec![0u8; size[0] as usize];
    data.read_exact(&mut bytes)?;
    Ok(ObjectValue::Raw(bytes))
}

fn read_text(data: &mut impl Read) -> Result<ObjectValue, Error> {
    let mut size = [0u8; 1];
    data.read_exact(&mut size)?;
    let mut bytes = vec![0u8; size[0] as usize];
    data.read_exact(&mut bytes)?;
    Ok(ObjectValue::Text(
        String::from_utf8(bytes).map_err(|_| Error::InvalidTextEncoding)?,
    ))
}

fn read_button_event(data: &mut impl Read) -> Result<ObjectValue, Error> {
    let mut bytes = [0u8; 1];
    data.read_exact(&mut bytes)?;
    Ok(ObjectValue::ButtonEvent(ButtonEvent::try_from(bytes[0])?))
}

fn read_dimmer_event(data: &mut impl Read) -> Result<ObjectValue, Error> {
    let mut bytes = [0u8; 2];
    data.read_exact(&mut bytes)?;
    Ok(ObjectValue::DimmerEvent(DimmerEvent::try_from(bytes[0])?, bytes[1]))
}

// Inspired by https://stackoverflow.com/questions/28028854/how-do-i-match-enum-values-with-an-integer
macro_rules! bthome_objects {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident($val:literal, $conv:path$(, $args:literal)?),)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname = $val,)*
        }

        impl std::convert::TryFrom<u8> for $name {
            type Error = Error;

            fn try_from(v: u8) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u8 => Ok($name::$vname),)*
                    _ => Err(Error::InvalidObjectId(v)),
                }
            }
        }

        fn value_from_raw(
            object_id: $name,
            data: &mut impl Read,
        ) -> Result<Object, Error> {
            let value = match object_id {
                $($name::$vname => $conv(data$(, $args)*)?,)*
            };
            Ok(Object {
                object_id,
                value,
            })
        }
    }
}

bthome_objects! {
#[repr(u8)]
#[derive(Debug)]
pub enum ObjectId {
    /* Sensor data */
    /// Unit: m/s² type: uint16 factor: 0.001
    Acceleration(0x51, float_from::uint16, 0.001),
    /// Unit: % type: uint8
    Battery(0x01, int_from::uint8),
    /// Unit: ppm type: uint16
    CO2(0x12, int_from::uint16),
    /// Unit: µS/cm type: uint16
    Conductivity(0x56, int_from::uint16),
    /// type: uint8
    CountU8(0x09, int_from::uint8),
    /// type: uint16
    CountU16(0x3D, int_from::uint16),
    /// type: uint32
    CountU32(0x3E, int_from::uint32),
    /// type: sint8
    CountI8(0x59, int_from::sint8),
    /// type: sint16
    CountI16(0x5A, int_from::sint16),
    /// type: sint32
    CountI32(0x5B, int_from::sint32),
    /// Unit: A type: uint16 factor: 0.001
    CurrentU16(0x43, float_from::uint16 , 0.001),
    /// Unit: A type: sint16 factor: 0.001
    CurrentI16(0x5D, float_from::sint16 , 0.001),
    /// Unit: °C type: sint16 factor: 0.01
    Dewpoint(0x08, float_from::sint16 , 0.01),
    /// Unit: mm type: uint16
    DistanceMM(0x40, int_from::uint16),
    /// Unit: m type: uint16 factor: 0.1
    DistanceM(0x41, float_from::uint16 , 0.1),
    /// Unit: s type: uint24 factor: 0.001
    Duration(0x42, float_from::uint24 , 0.001),
    /// Unit: kWh type: uint32 factor: 0.001
    EnergyU32(0x4D, float_from::uint32 , 0.001),
    /// Unit: kWh type: uint24 factor: 0.001
    EngergyU24(0x0A, float_from::uint24 , 0.001),
    /// Unit: m³ type: uint24 factor: 0.001
    GasU24(0x4B, float_from::uint24 , 0.001),
    /// Unit: m³ type: uint32 factor: 0.001
    GasU32(0x4C, float_from::uint32 , 0.001),
    /// Unit: °/s type: uint16 factor: 0.001
    Gyroscope(0x52, float_from::uint16 , 0.001),
    /// Unit: % type: uint16 factor: 0.01
    HumidityU16(0x03, float_from::uint16 , 0.01),
    /// Unit: % type: uint8
    HumidityU8(0x2E, int_from::uint8),
    /// Unit: lux type: uint24 factor: 0.01
    Illuminance(0x05, float_from::uint24 , 0.01),
    /// Unit: kg type: uint16 factor: 0.01
    MassKg(0x06, float_from::uint16 , 0.01),
    /// Unit: lb type: uint16 factor: 0.01
    MassLb(0x07, float_from::uint16 , 0.01),
    /// Unit: % type: uint16 factor: 0.01
    MoistureSmall(0x14, float_from::uint16 , 0.01),
    /// Unit: % type: uint8
    MoistureLarge(0x2F, int_from::uint8),
    /// Unit: µg/m³ type: uint16
    PM2d5(0x0D, int_from::uint16),
    /// Unit: µg/m³ type: uint16
    PM10(0x0E, int_from::uint16),
    /// Unit: W type: uint24 factor: 0.01
    PowerSmall(0x0B, float_from::uint24 , 0.01),
    /// Unit: W type: sint32 factor: 0.01
    PowerLarge(0x5C, float_from::sint32 , 0.01),
    /// Unit: hPa type: uint24 factor: 0.01
    Pressure(0x04, float_from::uint24 , 0.01),
    Raw(0x54, read_bytes),
    /// Unit: ° type: sint16 factor: 0.1
    Rotation(0x3F, float_from::sint16 , 0.1),
    /// Unit: m/s type: uint16 factor: 0.01
    Speed(0x44, float_from::uint16, 0.01),
    /// Unit: °C type: sint8
    Temperature1(0x57, int_from::sint8),
    /// Unit: °C type: sint8 factor: 0.35
    Temperature2(0x58, float_from::sint8 , 0.35),
    /// Unit: °C type: sint16 factor: 0.1
    Temperature3(0x45, float_from::sint16 , 0.1),
    /// Unit: °C type: sint16 factor: 0.01
    Temperature4(0x02, float_from::sint16 , 0.01),
    Text(0x53, read_text),
    /// Unit: s type: uint48
    Timestamp(0x50, int_from::uint48),
    /// Unit: µg/m³ type: uint16
    Tvoc(0x13, int_from::uint16),
    /// Unit: V type: uint16 factor: 0.001
    VoltageSmall(0x0C, float_from::uint16 , 0.001),
    /// Unit: V type: uint16 factor: 0.1
    VoltageLarge(0x4A, float_from::uint16 , 0.1),
    /// Unit: L type: uint32 factor: 0.001
    Volume1(0x4E, float_from::uint32 , 0.001),
    /// Unit: L type: uint16 factor: 0.1
    Volume2(0x47, float_from::uint16 , 0.1),
    /// Unit: mL type: uint16
    Volume3(0x48, int_from::uint16),
    /// Unit: L type: uint32 factor: 0.001
    VolumeStorage(0x55, float_from::uint32 , 0.001),
    /// Unit: m³/h type: uint16 factor: 0.001
    VolumeFlowRate(0x49, float_from::uint16 , 0.001),
    /// type: uint8 factor: 0.1
    UVIndex(0x46, float_from::uint8, 0.1),
    /// Unit: L type: uint32 factor: 0.001
    Water(0x4F, float_from::uint32 , 0.001),

    /* Binary sensor data */
    BatteryLow(0x15, read_bool),
    BatteryCharging(0x16, read_bool),
    CarbonMonoxideDetected(0x17, read_bool),
    Cold(0x18, read_bool),
    Connectivity(0x19, read_bool),
    DoorOpen(0x1A, read_bool),
    GarageDoorOpen(0x1B, read_bool),
    GasDetected(0x1C, read_bool),
    GenericBoolean(0x0F, read_bool),
    Heat(0x1D, read_bool),
    LightDetected(0x1E, read_bool),
    LockUnlocked(0x1F, read_bool),
    MoistureDetected(0x20, read_bool),
    MotionDetected(0x21, read_bool),
    MovementDetected(0x22, read_bool),
    OccupancyDetected(0x23, read_bool),
    IsOpen(0x11, read_bool),
    PluggedIn(0x24, read_bool),
    PowerOn(0x10, read_bool),
    PresenceAtHome(0x25, read_bool),
    ProblemDetected(0x26, read_bool),
    IsRunning(0x27, read_bool),
    IsSafe(0x28, read_bool),
    SmokeDetected(0x29, read_bool),
    SoundDetected(0x2A, read_bool),
    TamperDetected(0x2B, read_bool),
    VibrationDetected(0x2C, read_bool),
    WindowOpen(0x2D, read_bool),

    /* Events */
    Button(0x3A, read_button_event),
    Dimmer(0x3C, read_dimmer_event),

    /* Device information */
    DeviceTypeId(0xF0, int_from::uint16),
    FirmwareVersionLarge(0xF1, int_from::uint32),
    FirmwareVersionSmall(0xF2, int_from::uint64),

    /* Misc data */
    PacketId(0x00, int_from::uint8),
}
}

#[derive(Debug)]
pub enum ObjectValue {
    Float(f32),
    Int(i64),
    Bool(bool),
    Raw(Vec<u8>),
    ButtonEvent(ButtonEvent),
    DimmerEvent(DimmerEvent, u8),
    Text(String),
}

#[derive(Debug)]
pub struct Object {
    pub object_id: ObjectId,
    pub value: ObjectValue,
}

#[derive(Debug)]
pub struct ServiceData {
    pub encrypted: bool,
    pub trigger_based: bool,
    pub version: u8,
    pub objects: Vec<Object>,
}

impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::IoError(value)
    }
}

pub fn parse_service_data(data: &[u8]) -> Result<ServiceData, Error> {
    let mut cursor = Cursor::new(data);
    let mut head = [0u8];
    cursor.read_exact(&mut head)?;
    let mut service_data = ServiceData {
        encrypted: head[0] & 0b00000001 == 1,
        trigger_based: head[0] & 0b00000100 == 1,
        version: head[0] >> 5,
        objects: Vec::new(),
    };
    loop {
        let mut next_byte = [0u8];
        if let Err(err) = cursor.read_exact(&mut next_byte) {
            if err.kind() == std::io::ErrorKind::UnexpectedEof {
                break;
            } else {
                return Err(Error::IoError(err));
            }
        }
        let object_id = ObjectId::try_from(next_byte[0])?;
        service_data
            .objects
            .push(value_from_raw(object_id, &mut cursor)?);
    }
    Ok(service_data)
}
