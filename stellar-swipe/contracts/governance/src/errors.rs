use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum GovernanceError {
    AlreadyInitialized = 1,
    NotInitialized = 2,
    Unauthorized = 3,
    InvalidSupply = 4,
    InvalidAmount = 5,
    InvalidDuration = 6,
    DuplicateSchedule = 7,
    VestingScheduleNotFound = 8,
    CliffNotReached = 9,
    NothingToRelease = 10,
    InsufficientBalance = 11,
    InsufficientStakedBalance = 12,
    ActiveVoteLock = 13,
    BelowMinimumClaim = 14,
    LiquidityPoolExhausted = 15,
    DuplicateRecipient = 16,
    ArithmeticOverflow = 17,
    InvalidRewardConfig = 18,
    InvalidMetadata = 19,
}
