//! Demo-facing physical connector descriptions and channel routing policy.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ControllerFamily {
    Xbox360,
    DualSense,
    DualShock4,
    SwitchPro,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JackConnector {
    Mm25,
    Mm35,
}

impl JackConnector {
    pub const fn millimeters(self) -> &'static str {
        match self {
            Self::Mm25 => "2.5",
            Self::Mm35 => "3.5",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum JackDevice {
    MonoSpeaker,
    StereoSpeaker,
    Microphone,
    Headset,
}

impl JackDevice {
    pub const ALL: [Self; 4] = [
        Self::MonoSpeaker,
        Self::StereoSpeaker,
        Self::Microphone,
        Self::Headset,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::MonoSpeaker => "Mono speaker",
            Self::StereoSpeaker => "Stereo speaker",
            Self::Microphone => "Microphone only",
            Self::Headset => "TRRS headset",
        }
    }

    pub const fn label_for_connector(self, connector: JackConnector) -> &'static str {
        match (self, connector) {
            (Self::Headset, JackConnector::Mm35) => "TRRS headset",
            (Self::Headset, JackConnector::Mm25) => "Wired headset",
            _ => self.label(),
        }
    }

    pub const fn output_channels(self) -> usize {
        match self {
            Self::MonoSpeaker => 1,
            Self::StereoSpeaker | Self::Headset => 2,
            Self::Microphone => 0,
        }
    }

    pub const fn input_channels(self) -> usize {
        match self {
            Self::Microphone | Self::Headset => 1,
            Self::MonoSpeaker | Self::StereoSpeaker => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AudioTopology {
    pub onboard_speaker_channels: usize,
    pub onboard_microphone_channels: usize,
    pub jack: Option<JackConnector>,
}

impl AudioTopology {
    pub const fn for_family(family: ControllerFamily) -> Self {
        match family {
            ControllerFamily::Xbox360 => Self {
                onboard_speaker_channels: 0,
                onboard_microphone_channels: 0,
                jack: Some(JackConnector::Mm25),
            },
            ControllerFamily::DualSense => Self {
                onboard_speaker_channels: 1,
                onboard_microphone_channels: 2,
                jack: Some(JackConnector::Mm35),
            },
            ControllerFamily::DualShock4 => Self {
                onboard_speaker_channels: 1,
                onboard_microphone_channels: 0,
                jack: Some(JackConnector::Mm35),
            },
            ControllerFamily::SwitchPro => Self {
                onboard_speaker_channels: 0,
                onboard_microphone_channels: 0,
                jack: None,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct ChannelRoute {
    /// `None` means the destination channel is silent.
    pub source: Option<usize>,
    pub destination: usize,
}

/// Match left and right channels by their conventional positions, then match
/// remaining channels by index. Unmatched destination channels stay silent.
#[cfg(test)]
pub(super) fn default_channel_routes(
    source_count: usize,
    destination_count: usize,
) -> Vec<ChannelRoute> {
    (0..destination_count)
        .map(|destination| ChannelRoute {
            source: (destination < source_count).then_some(destination),
            destination,
        })
        .collect()
}

/// Find usable stereo pairs, matching semantic left/right names before using
/// adjacent channels as a fallback. Haptic endpoints never become speaker sources.
pub(super) fn stereo_channel_pairs(labels: &[String]) -> Vec<(usize, usize)> {
    let usable = labels
        .iter()
        .enumerate()
        .filter(|(_, label)| !label.to_ascii_lowercase().contains("haptic"))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let mut pairs = Vec::new();
    let mut used = std::collections::HashSet::new();

    for &left in &usable {
        let lower = labels[left].to_ascii_lowercase();
        let Some(position) = lower.find("left") else {
            continue;
        };
        let right_label = format!("{}right{}", &lower[..position], &lower[position + 4..]);
        if let Some(right) = usable.iter().copied().find(|&right| {
            !used.contains(&right) && labels[right].eq_ignore_ascii_case(&right_label)
        }) {
            pairs.push((left, right));
            used.insert(left);
            used.insert(right);
        }
    }

    for pair in usable
        .into_iter()
        .filter(|index| !used.contains(index))
        .collect::<Vec<_>>()
        .chunks_exact(2)
    {
        pairs.push((pair[0], pair[1]));
    }
    pairs
}

pub(super) fn apply_channel_routes(
    source: &[i16],
    source_channels: usize,
    destination: &mut [i16],
    destination_channels: usize,
    routes: &[ChannelRoute],
) {
    if source_channels == 0 || destination_channels == 0 {
        destination.fill(0);
        return;
    }
    let frames = (source.len() / source_channels).min(destination.len() / destination_channels);
    destination.fill(0);
    for route in routes {
        if route.destination >= destination_channels {
            continue;
        }
        let Some(source_channel) = route.source.filter(|source| *source < source_channels) else {
            continue;
        };
        for frame in 0..frames {
            destination[frame * destination_channels + route.destination] =
                source[frame * source_channels + source_channel];
        }
    }
}

pub(super) fn mix_channel_routes(
    source: &[i16],
    source_channels: usize,
    destination: &mut [i16],
    destination_channels: usize,
    routes: &[ChannelRoute],
) -> Vec<u16> {
    if source_channels == 0 || destination_channels == 0 {
        return Vec::new();
    }
    let frames = (source.len() / source_channels).min(destination.len() / destination_channels);
    for route in routes {
        if route.destination >= destination_channels {
            continue;
        }
        let Some(source_channel) = route.source.filter(|source| *source < source_channels) else {
            continue;
        };
        for frame in 0..frames {
            let source_sample = source[frame * source_channels + source_channel];
            let destination_sample =
                &mut destination[frame * destination_channels + route.destination];
            *destination_sample = destination_sample.saturating_add(source_sample);
        }
    }
    (0..destination_channels)
        .map(|channel| {
            destination
                .iter()
                .skip(channel)
                .step_by(destination_channels)
                .map(|sample| sample.unsigned_abs())
                .max()
                .unwrap_or(0)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn topology_lists_only_physical_audio_features() {
        assert_eq!(
            AudioTopology::for_family(ControllerFamily::Xbox360),
            AudioTopology {
                onboard_speaker_channels: 0,
                onboard_microphone_channels: 0,
                jack: Some(JackConnector::Mm25),
            }
        );
        assert_eq!(
            AudioTopology::for_family(ControllerFamily::DualSense),
            AudioTopology {
                onboard_speaker_channels: 1,
                onboard_microphone_channels: 2,
                jack: Some(JackConnector::Mm35),
            }
        );
        assert_eq!(
            AudioTopology::for_family(ControllerFamily::DualShock4),
            AudioTopology {
                onboard_speaker_channels: 1,
                onboard_microphone_channels: 0,
                jack: Some(JackConnector::Mm35),
            }
        );
        assert_eq!(
            AudioTopology::for_family(ControllerFamily::SwitchPro),
            AudioTopology {
                onboard_speaker_channels: 0,
                onboard_microphone_channels: 0,
                jack: None,
            }
        );
    }

    #[test]
    fn default_channel_routing_supports_arbitrary_channel_counts() {
        let routes = default_channel_routes(2, 4);
        assert_eq!(
            routes,
            [
                ChannelRoute {
                    source: Some(0),
                    destination: 0
                },
                ChannelRoute {
                    source: Some(1),
                    destination: 1
                },
                ChannelRoute {
                    source: None,
                    destination: 2
                },
                ChannelRoute {
                    source: None,
                    destination: 3
                },
            ]
        );
        assert_eq!(default_channel_routes(8, 12).len(), 12);
        assert_eq!(default_channel_routes(12, 8).len(), 8);
    }

    #[test]
    fn stereo_pairs_match_left_right_semantics_and_ignore_haptics() {
        let labels =
            ["AudibleLeft", "AudibleRight", "HapticLeft", "HapticRight"].map(str::to_owned);
        assert_eq!(stereo_channel_pairs(&labels), [(0, 1)]);

        let labels = ["Channel 1", "Channel 2", "Channel 3"].map(str::to_owned);
        assert_eq!(stereo_channel_pairs(&labels), [(0, 1)]);
    }

    #[test]
    fn channel_overrides_route_or_silence_any_destination_channel() {
        let source = [10, 11, 20, 21];
        let mut destination = [0; 6];
        apply_channel_routes(
            &source,
            2,
            &mut destination,
            3,
            &[
                ChannelRoute {
                    source: Some(1),
                    destination: 0,
                },
                ChannelRoute {
                    source: None,
                    destination: 1,
                },
                ChannelRoute {
                    source: Some(0),
                    destination: 2,
                },
            ],
        );
        assert_eq!(destination, [11, 0, 10, 21, 0, 20]);
    }

    #[test]
    fn independent_microphone_sources_mix_without_clipping_and_keep_n_channels() {
        let mut destination = [0_i16; 4 * 2];
        let peaks = mix_channel_routes(
            &[12, -12, 24, -24],
            2,
            &mut destination,
            4,
            &[
                ChannelRoute {
                    source: Some(0),
                    destination: 0,
                },
                ChannelRoute {
                    source: Some(1),
                    destination: 3,
                },
            ],
        );
        assert_eq!(destination, [12, 0, 0, -12, 24, 0, 0, -24]);
        assert_eq!(peaks, [24, 0, 0, 24]);
    }

    #[test]
    fn jack_modes_declare_independent_input_and_output_channels() {
        assert_eq!(JackDevice::MonoSpeaker.output_channels(), 1);
        assert_eq!(JackDevice::StereoSpeaker.output_channels(), 2);
        assert_eq!(JackDevice::Microphone.input_channels(), 1);
        assert_eq!(JackDevice::Headset.output_channels(), 2);
        assert_eq!(JackDevice::Headset.input_channels(), 1);
    }
}
