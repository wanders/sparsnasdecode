//! Decoder for packets generated by IKEA Sparsnas enegry meter
//!
//! See <https://github.com/kodarn/Sparsnas> for a very detailed
//! reverse engineering of the protocol.

mod ikeacrc;

pub struct SparsnasDecoder {
    serial: u32,
    key: [u8; 5],
}

#[derive(Debug, PartialEq)]
pub struct SparsnasPacket {
    /// Sequence number for this packet. The transmitter increments
    /// this for each packet it sends.
    pub packet_seq: u16,

    /// Current time between pulses. This can be used to calculate current power usage. Use [SparsnasPacket::power] function to do that calculation.
    pub time_between_pulses: u16,

    /// Total number of pulses (blinks) transmitter has seen since poweron.
    pub pulse_count: u32,

    ///
    pub battery_percentage: u8,

    ///
    pub status: u16,

    /// This is the last 6 (decimal) digits of the serial number of the transmitter.
    pub serial: u32,
}

#[derive(Debug, PartialEq)]
pub enum SparsnasDecodeError {
    BadCRC,
    BadLength,
    BadSerial,
    BadPacketCount,
}

impl SparsnasPacket {
    /// Calculate and return power usage as reported in the packet.
    ///
    /// `pulses_per_khw`: The number of pulses the meter gives per kWh. (usually 1000)
    pub fn power(&self, pulses_per_khw: u32) -> u32 {
        3686400000u32 / (pulses_per_khw * self.time_between_pulses as u32)
    }
}

impl SparsnasDecoder {
    /// Create a new decoder for specified serial number.
    ///
    /// The serial number is on a label behind the batteries, 9 digits
    /// (nnn-nnn-nnn)
    pub fn new(serial: u32) -> Self {
        let xorbase = (serial + 0x8AEF9335).to_le_bytes();
        SparsnasDecoder {
            serial,
            key: [0x47, xorbase[2], xorbase[3], xorbase[0], xorbase[1]],
        }
    }

    /// Decode a packet without CRC.
    pub fn decode_nocrc(&self, data: &[u8; 18]) -> Result<SparsnasPacket, SparsnasDecodeError> {
        if data[0] != 17 {
            return Err(SparsnasDecodeError::BadLength);
        }

        let pkt = SparsnasPacket {
            status: u16::from_be_bytes([data[3] ^ self.key[0], data[4] ^ self.key[1]]),
            serial: u32::from_be_bytes([
                data[5] ^ self.key[2],
                data[6] ^ self.key[3],
                data[7] ^ self.key[4],
                data[8] ^ self.key[0],
            ]),
            packet_seq: u16::from_be_bytes([data[9] ^ self.key[1], data[10] ^ self.key[2]]),
            time_between_pulses: u16::from_be_bytes([
                data[11] ^ self.key[3],
                data[12] ^ self.key[4],
            ]),
            pulse_count: u32::from_be_bytes([
                data[13] ^ self.key[0],
                data[14] ^ self.key[1],
                data[15] ^ self.key[2],
                data[16] ^ self.key[3],
            ]),
            battery_percentage: data[17] ^ self.key[4],
        };

        if (pkt.packet_seq & 0x7f) as u8 != data[2] {
            return Err(SparsnasDecodeError::BadPacketCount);
        }

        if pkt.serial != self.serial % 1_000_000 {
            return Err(SparsnasDecodeError::BadSerial);
        }

        Ok(pkt)
    }

    /// Decode a packet. Expecting that the buffer contains a CRC at the end.
    pub fn decode(&self, data: &[u8; 20]) -> Result<SparsnasPacket, SparsnasDecodeError> {
        let crc = ikeacrc::crc(&data[0..18]);

        if u16::from_be_bytes([data[18], data[19]]) != crc {
            return Err(SparsnasDecodeError::BadCRC);
        }

        self.decode_nocrc(data[0..18].try_into().unwrap())
    }
}

#[cfg(test)]
mod tests {

    use crate::*;

    #[test]
    fn kodarn() {
        // from https://github.com/kodarn/Sparsnas
        let testdata = [
            0x11, 0x49, 0x24, 0x07, 0x0e, 0xa2, 0x76, 0x17, 0x0e, 0xcf, 0x86, 0x91, 0x67, 0x47,
            0xcf, 0xa2, 0x77, 0xd3, 0x6e, 0x2d,
        ];

        let d = SparsnasDecoder::new(400_565_321);

        let pkt = d.decode(&testdata).unwrap();
        let pkt_no_crc = d.decode_nocrc(testdata[0..18].try_into().unwrap()).unwrap();

        let expected = SparsnasPacket {
            packet_seq: 36,
            time_between_pulses: 61392,
            pulse_count: 9,
            battery_percentage: 100,
            status: 16577,
            serial: 565321,
        };

        assert_eq!(pkt, expected);
        assert_eq!(pkt_no_crc, expected);
    }

    #[test]
    fn real() {
        let testdata = [
            0x11, 0xe0, 0x2b, 0x07, 0x0e, 0xa2, 0x1d, 0x28, 0xa7, 0x80, 0x09, 0x12, 0xbe, 0x47,
            0x8a, 0x20, 0x5b, 0x14, 0x69, 0x57,
        ];

        let d = SparsnasDecoder::new(400_547_040);

        let pkt = d.decode(&testdata).unwrap();

        let expected = SparsnasPacket {
            packet_seq: 20395,
            time_between_pulses: 1998,
            pulse_count: 4555342,
            battery_percentage: 100,
            status: 16577,
            serial: 547040,
        };

        assert_eq!(pkt, expected);
        assert_eq!(pkt.power(1000), 1845);
    }

    #[test]
    fn bad_crc() {
        let testdata = [
            0x11, 0xe0, 0x2b, 0x07, 0x0e, 0xa2, 0x1d, 0x28, 0xa7, 0x80, 0x09, 0x12, 0xbe, 0x47,
            0x8a, 0x20, 0x5b, 0x14, 0xff, 0xff, // last two u8 crc changed
        ];

        let d = SparsnasDecoder::new(400_547_040);

        let res = d.decode(&testdata);

        assert_eq!(res, Err(SparsnasDecodeError::BadCRC));
    }
}