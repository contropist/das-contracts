use core::convert::TryFrom;

pub type DymLibSize = [u8; 128 * 1024];

#[derive(Debug, PartialEq, Copy, Clone)]
#[repr(u8)]
pub enum DasLockType {
    XXX,
    CKBMulti,
    CKBSingle,
    ETH,
    TRON,
    ETHTypedData,
    MIXIN,
}

impl TryFrom<u8> for DasLockType {
    type Error = ();

    fn try_from(v: u8) -> Result<Self, Self::Error> {
        match v {
            x if x == DasLockType::XXX as u8 => Ok(DasLockType::XXX),
            x if x == DasLockType::CKBMulti as u8 => Ok(DasLockType::CKBMulti),
            x if x == DasLockType::CKBSingle as u8 => Ok(DasLockType::CKBSingle),
            x if x == DasLockType::ETH as u8 => Ok(DasLockType::ETH),
            x if x == DasLockType::TRON as u8 => Ok(DasLockType::TRON),
            x if x == DasLockType::ETHTypedData as u8 => Ok(DasLockType::ETHTypedData),
            x if x == DasLockType::MIXIN as u8 => Ok(DasLockType::MIXIN),
            _ => Err(()),
        }
    }
}

#[cfg(feature = "mainnet")]
pub const CKB_MULTI_LIB_CODE_HASH: [u8; 32] = [
    199, 227, 155, 255, 158, 1, 22, 72, 63, 199, 114, 10, 103, 174, 212, 50, 184, 70, 72, 221, 243, 10, 250, 95, 181,
    118, 172, 55, 143, 199, 98, 66,
];

#[cfg(not(feature = "mainnet"))]
pub const CKB_MULTI_LIB_CODE_HASH: [u8; 32] = [
    103, 34, 170, 109, 16, 228, 36, 225, 200, 32, 117, 90, 105, 190, 113, 36, 46, 167, 229, 138, 143, 115, 94, 145, 61,
    152, 187, 231, 33, 188, 236, 226,
];

#[cfg(feature = "mainnet")]
pub const ETH_LIB_CODE_HASH: [u8; 32] = [
    184, 112, 42, 157, 136, 93, 85, 232, 246, 244, 116, 198, 101, 0, 175, 16, 170, 14, 254, 155, 121, 55, 246, 120, 95,
    130, 7, 63, 200, 42, 60, 11,
];

#[cfg(not(feature = "mainnet"))]
pub const ETH_LIB_CODE_HASH: [u8; 32] = [
    114, 136, 18, 7, 241, 131, 151, 251, 114, 137, 71, 94, 28, 208, 216, 64, 104, 55, 4, 5, 126, 140, 166, 6, 43, 114,
    139, 209, 174, 122, 155, 68,
];

#[cfg(feature = "mainnet")]
pub const TRON_LIB_CODE_HASH: [u8; 32] = [
    208, 23, 88, 157, 118, 11, 50, 132, 8, 19, 88, 141, 78, 193, 52, 163, 252, 203, 1, 3, 28, 140, 214, 85, 178, 139,
    120, 33, 87, 192, 215, 137,
];

#[cfg(not(feature = "mainnet"))]
pub const TRON_LIB_CODE_HASH: [u8; 32] = [
    170, 97, 164, 212, 192, 24, 68, 18, 215, 238, 129, 129, 59, 215, 28, 198, 72, 222, 68, 16, 49, 230, 111, 167, 153,
    172, 66, 113, 180, 208, 117, 131,
];