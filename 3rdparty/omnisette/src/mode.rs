/// Optional remote authentication profiles matching iloader v2.4.0.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(
    feature = "remote-anisette-v3",
    derive(serde::Serialize, serde::Deserialize)
)]
#[cfg_attr(feature = "remote-anisette-v3", serde(rename_all = "kebab-case"))]
pub enum AnisetteMode {
    #[default]
    Standard,
    Iloader,
    IloaderSidestore,
    IloaderStikstore,
}

impl AnisetteMode {
    pub const ALL: [Self; 4] = [
        Self::Standard,
        Self::Iloader,
        Self::IloaderSidestore,
        Self::IloaderStikstore,
    ];

    pub fn server_url(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Iloader => Some("https://mac-ani.jkcoxson.com"),
            Self::IloaderSidestore => Some("https://new.ani.sidestore.io"),
            Self::IloaderStikstore => Some("https://ani3.stikstore.app"),
        }
    }

    pub fn cache_directory(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Iloader => Some("iloader-jkcoxson"),
            Self::IloaderSidestore => Some("iloader-sidestore"),
            Self::IloaderStikstore => Some("iloader-stikstore"),
        }
    }

    pub fn uses_server_identity(self) -> bool {
        self != Self::Standard
    }
}

impl std::str::FromStr for AnisetteMode {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "standard" => Ok(Self::Standard),
            "iloader" => Ok(Self::Iloader),
            "iloader-sidestore" => Ok(Self::IloaderSidestore),
            "iloader-stikstore" => Ok(Self::IloaderStikstore),
            _ => Err(format!(
                "Unknown anisette mode: {value}. Use standard, iloader, iloader-sidestore or iloader-stikstore."
            )),
        }
    }
}
