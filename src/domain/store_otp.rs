use chrono::NaiveDateTime;

/// Domain representation of an OTP request associated with a storefront customer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOtp {
    /// Hub identifier that owns the OTP record.
    pub hub_id: i32,
    /// Phone number the OTP was generated for.
    pub phone: String,
    /// Six-digit OTP code.
    pub code: String,
    /// Timestamp indicating when the OTP expires.
    pub expires_at: NaiveDateTime,
    /// Timestamp capturing when the OTP was last sent.
    pub last_sent_at: NaiveDateTime,
}

/// Payload used to persist a new or updated OTP record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewStoreOtp {
    /// Hub identifier that owns the OTP record.
    pub hub_id: i32,
    /// Phone number the OTP targets.
    pub phone: String,
    /// Six-digit OTP code.
    pub code: String,
    /// Timestamp indicating when the OTP expires.
    pub expires_at: NaiveDateTime,
    /// Timestamp capturing when the OTP was last sent.
    pub last_sent_at: NaiveDateTime,
}

impl NewStoreOtp {
    /// Construct a new OTP payload from sanitised inputs.
    #[must_use]
    pub fn new(
        hub_id: i32,
        phone: impl Into<String>,
        code: impl Into<String>,
        expires_at: NaiveDateTime,
        last_sent_at: NaiveDateTime,
    ) -> Self {
        Self {
            hub_id,
            phone: phone.into(),
            code: code.into(),
            expires_at,
            last_sent_at,
        }
    }
}
