use soroban_sdk::contracterror;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum AutoTradeError {
    InvalidAmount = 1,
    Unauthorized = 2,
    SignalNotFound = 3,
    SignalExpired = 4,
    InsufficientBalance = 5,
    InsufficientLiquidity = 6,
    DailyTradeLimitExceeded = 7,
    PositionLimitExceeded = 8,
    StopLossTriggered = 9,
    TradingPaused = 10,

    StrategyNotFound = 11,
    PositionAlreadyExists = 12,
    RankingDisabled = 13,

    InvalidBasketSize = 14,
    InsufficientPriceHistory = 15,
    InvalidPriceData = 16,
    NonCointegratedBasket = 17,
    ActivePortfolioExists = 18,
    NoActivePortfolio = 19,
    NoTradeSignal = 20,
    InvalidStatArbConfig = 21,
    
    // Pairs Trading
    PairsStrategyNotFound = 22,
    PairsActivePositionExists = 23,
    PairsNoActivePosition = 24,
    InsufficientCorrelation = 25,
    PairNotCointegrated = 26,
    InvalidPairsConfig = 27,
 
    // Arbitrage
    ArbitrageOpportunityExpired = 28,
    ArbitrageUnprofitable = 29,
    ArbTooLarge = 30,
    FrontRunningRisk = 31,

    // Insurance
    InvalidInsuranceConfig = 32,
    InsuranceNotConfigured = 33,

    // Referral
    SelfReferral = 34,
    ReferralAlreadySet = 35,
    CircularReferral = 36,
    ReferralLimitExceeded = 37,
}
