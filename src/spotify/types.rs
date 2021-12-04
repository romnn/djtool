
#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Debug, AsRefStr)]
pub enum CopyrightType {
    #[strum(serialize = "P")]
    #[serde(rename = "P")]
    Performance,
    #[strum(serialize = "C")]
    #[serde(rename = "C")]
    Copyright,
}

#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Debug, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AlbumType {
    Album,
    Single,
    AppearsOn,
    Compilation,
}

#[derive(
    Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Debug, Display, EnumString, AsRefStr,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Type {
    Artist,
    Album,
    Track,
    Playlist,
    User,
    Show,
    Episode,
}

#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Debug, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum AdditionalType {
    Track,
    Episode,
}

#[derive(Clone, Serialize, Deserialize, Copy, PartialEq, Eq, Debug, AsRefStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum CurrentlyPlayingType {
    Track,
    Episode,
    #[strum(serialize = "ad")]
    #[serde(rename = "ad")]
    Advertisement,
    Unknown,
}
