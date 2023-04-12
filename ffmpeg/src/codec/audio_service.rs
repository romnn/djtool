// use ffi::AVAudioServiceType::*;
use crate::ffi::*;

#[derive(Eq, PartialEq, Clone, Copy, Debug)]
pub enum AudioService {
    Main,
    Effects,
    VisuallyImpaired,
    HearingImpaired,
    Dialogue,
    Commentary,
    Emergency,
    VoiceOver,
    Karaoke,
}

impl From<AVAudioServiceType> for AudioService {
    fn from(value: AVAudioServiceType) -> Self {
        match value {
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_MAIN => AudioService::Main,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_EFFECTS => AudioService::Effects,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED => {
                AudioService::VisuallyImpaired
            }
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED => {
                AudioService::HearingImpaired
            }
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_DIALOGUE => AudioService::Dialogue,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_COMMENTARY => AudioService::Commentary,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_EMERGENCY => AudioService::Emergency,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_VOICE_OVER => AudioService::VoiceOver,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_KARAOKE => AudioService::Karaoke,
            AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_NB => AudioService::Main,
        }
    }
}

impl From<AudioService> for AVAudioServiceType {
    fn from(value: AudioService) -> AVAudioServiceType {
        match value {
            AudioService::Main => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_MAIN,
            AudioService::Effects => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_EFFECTS,
            AudioService::VisuallyImpaired => {
                AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_VISUALLY_IMPAIRED
            }
            AudioService::HearingImpaired => {
                AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_HEARING_IMPAIRED
            }
            AudioService::Dialogue => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_DIALOGUE,
            AudioService::Commentary => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_COMMENTARY,
            AudioService::Emergency => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_EMERGENCY,
            AudioService::VoiceOver => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_VOICE_OVER,
            AudioService::Karaoke => AVAudioServiceType::AV_AUDIO_SERVICE_TYPE_KARAOKE,
        }
    }
}
