//! Defines the [`Props`] type, a key-value map that is used to associate arbitrary properties with
//! objects.

use std::{
    collections::BTreeMap,
    ffi::{CStr, CString},
};

use super::*;
use crate::protocol::ProtocolError;

/// Max. size of a proplist value in Bytes.
const MAX_PROP_SIZE: u32 = 64 * 1024;

/// A list of key-value pairs that associate arbitrary properties with an
/// object. Keys are null-terminated strings and values are arbitrary binary
/// blobs, although by convention both are usually null-terminated ASCII
/// strings.
#[derive(Default, Clone, PartialEq, Eq)]
pub struct Props(BTreeMap<Box<CStr>, Box<[u8]>>);

impl Props {
    /// Creates a new, empty property list.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a well-known property in the map.
    ///
    /// If the property already has a value, it will be overwritten with the new one.
    pub fn set<T>(&mut self, prop: Prop, value: T)
    where
        T: AsRef<CStr>,
    {
        self.set_bytes(prop.to_c_str(), value.as_ref().to_bytes_with_nul());
    }

    /// Sets a a property in the map.
    ///
    /// If the property already has a value, it will be overwritten with the new one.
    pub fn set_bytes<K, V>(&mut self, key: K, value: V)
    where
        K: AsRef<CStr>,
        V: AsRef<[u8]>,
    {
        self.0.insert(key.as_ref().into(), value.as_ref().into());
    }

    /// Gets the value of a well-known property.
    ///
    /// If `prop` is not in the map, returns `None`.
    pub fn get(&self, prop: Prop) -> Option<&[u8]> {
        self.get_bytes(prop.to_c_str())
    }

    /// Gets the value of a well-known property.
    ///
    /// If `prop` is not in the map, returns `None`.
    pub fn get_mut(&mut self, prop: Prop) -> Option<&mut [u8]> {
        self.get_bytes_mut(prop.to_c_str())
    }

    /// Gets a property from the map.
    pub fn get_bytes<K>(&self, key: K) -> Option<&[u8]>
    where
        K: AsRef<CStr>,
    {
        self.0.get(key.as_ref()).map(|r| &r[..])
    }

    ///s Get a property from the map.
    pub fn get_bytes_mut<K>(&mut self, key: K) -> Option<&mut [u8]>
    where
        K: AsRef<CStr>,
    {
        self.0.get_mut(key.as_ref()).map(|r| &mut r[..])
    }

    /// Create an Iterator over the properties.
    pub fn iter(&self) -> std::collections::btree_map::Iter<'_, Box<CStr>, Box<[u8]>> {
        self.0.iter()
    }
}

impl std::fmt::Debug for Props {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dm = f.debug_map();
        let invalid = CString::new("<bytes>").unwrap();

        for (k, v) in self.0.iter() {
            match CStr::from_bytes_with_nul(v) {
                Ok(s) => dm.entry(k, &s),
                Err(_) => dm.entry(k, &invalid),
            };
        }

        dm.finish()
    }
}

/// Well-known property list keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Prop {
    /// For streams: localized media name, formatted as UTF-8. E.g. "Guns'N'Roses: Civil War".
    MediaName,

    /// For streams: localized media title if applicable, formatted as UTF-8. E.g. "Civil War"
    MediaTitle,

    /// For streams: localized media artist if applicable, formatted as UTF-8. E.g. "Guns'N'Roses"
    MediaArtist,

    /// For streams: localized media copyright string if applicable, formatted as UTF-8. E.g. "Evil Record Corp."
    MediaCopyright,

    /// For streams: localized media generator software string if applicable, formatted as UTF-8. E.g. "Foocrop AudioFrobnicator"
    MediaSoftware,

    /// For streams: media language if applicable, in standard POSIX format. E.g. "de_DE"
    MediaLanguage,

    /// For streams: source filename if applicable, in URI format or local path. E.g. "/home/lennart/music/foobar.ogg"
    MediaFilename,

    /// For streams: icon for the media. A binary blob containing PNG image data
    MediaIcon,

    /// For streams: an XDG icon name for the media. E.g. "audio-x-mp3"
    MediaIconName,

    /// For streams: logic role of this media. One of the strings "video", "music", "game", "event", "phone", "animation", "production", "a11y", "test"
    MediaRole,

    /// For streams: the name of a filter that is desired, e.g.\ "echo-cancel" or "equalizer-sink". PulseAudio may choose to not apply the filter if it does not make sense (for example, applying echo-cancellation on a Bluetooth headset probably does not make sense. \since 1.0
    FilterWant,

    /// For streams: the name of a filter that is desired, e.g.\ "echo-cancel" or "equalizer-sink". Differs from PA_PROP_FILTER_WANT in that it forces PulseAudio to apply the filter, regardless of whether PulseAudio thinks it makes sense to do so or not. If this is set, PA_PROP_FILTER_WANT is ignored. In other words, you almost certainly do not want to use this. \since 1.0
    FilterApply,

    /// For streams: the name of a filter that should specifically suppressed (i.e.\ overrides PA_PROP_FILTER_WANT). Useful for the times that PA_PROP_FILTER_WANT is automatically added (e.g. echo-cancellation for phone streams when $VOIP_APP does its own, internal AEC) \since 1.0
    FilterSuppress,

    /// For event sound streams: XDG event sound name. e.g.\ "message-new-email" (Event sound streams are those with media.role set to "event")
    EventId,

    /// For event sound streams: localized human readable one-line description of the event, formatted as UTF-8. E.g. "Email from lennart@example.com received."
    EventDescription,

    /// For event sound streams: absolute horizontal mouse position on the screen if the event sound was triggered by a mouse click, integer formatted as text string. E.g. "865"
    EventMouseX,

    /// For event sound streams: absolute vertical mouse position on the screen if the event sound was triggered by a mouse click, integer formatted as text string. E.g. "432"
    EventMouseY,

    /// For event sound streams: relative horizontal mouse position on the screen if the event sound was triggered by a mouse click, float formatted as text string, ranging from 0.0 (left side of the screen) to 1.0 (right side of the screen). E.g. "0.65"
    EventMouseHPos,

    /// For event sound streams: relative vertical mouse position on the screen if the event sound was triggered by a mouse click, float formatted as text string, ranging from 0.0 (top of the screen) to 1.0 (bottom of the screen). E.g. "0.43"
    EventMouseVPos,

    /// For event sound streams: mouse button that triggered the event if applicable, integer formatted as string with 0=left, 1=middle, 2=right. E.g. "0"
    EventMouseButton,

    /// For streams that belong to a window on the screen: localized window title. E.g. "Totem Music Player"
    WindowName,

    /// For streams that belong to a window on the screen: a textual id for identifying a window logically. E.g. "org.gnome.Totem.MainWindow"
    WindowId,

    /// For streams that belong to a window on the screen: window icon. A binary blob containing PNG image data
    WindowIcon,

    /// For streams that belong to a window on the screen: an XDG icon name for the window. E.g. "totem"
    WindowIconName,

    /// For streams that belong to a window on the screen: absolute horizontal window position on the screen, integer formatted as text string. E.g. "865". \since 0.9.17
    WindowX,

    /// For streams that belong to a window on the screen: absolute vertical window position on the screen, integer formatted as text string. E.g. "343". \since 0.9.17
    WindowY,

    /// For streams that belong to a window on the screen: window width on the screen, integer formatted as text string. e.g. "365". \since 0.9.17
    WindowWidth,

    /// For streams that belong to a window on the screen: window height on the screen, integer formatted as text string. E.g. "643". \since 0.9.17
    WindowHeight,

    /// For streams that belong to a window on the screen: relative position of the window center on the screen, float formatted as text string, ranging from 0.0 (left side of the screen) to 1.0 (right side of the screen). E.g. "0.65". \since 0.9.17
    WindowHPos,

    /// For streams that belong to a window on the screen: relative position of the window center on the screen, float formatted as text string, ranging from 0.0 (top of the screen) to 1.0 (bottom of the screen). E.g. "0.43". \since 0.9.17
    WindowVPos,

    /// For streams that belong to a window on the screen: if the windowing system supports multiple desktops, a comma separated list of indexes of the desktops this window is visible on. If this property is an empty string, it is visible on all desktops (i.e. 'sticky'). The first desktop is 0. E.g. "0,2,3" \since 0.9.18
    WindowDesktop,

    /// For streams that belong to an X11 window on the screen: the X11 display string. E.g. ":0.0"
    WindowX11Display,

    /// For streams that belong to an X11 window on the screen: the X11 screen the window is on, an integer formatted as string. E.g. "0"
    WindowX11Screen,

    /// For streams that belong to an X11 window on the screen: the X11 monitor the window is on, an integer formatted as string. E.g. "0"
    WindowX11Monitor,

    /// For streams that belong to an X11 window on the screen: the window XID, an integer formatted as string. E.g. "25632"
    WindowX11Xid,

    /// For clients/streams: localized human readable application name. E.g. "Totem Music Player"
    ApplicationName,

    /// For clients/streams: a textual id for identifying an application logically. E.g. "org.gnome.Totem"
    ApplicationId,

    /// For clients/streams: a version string, e.g.\ "0.6.88"
    ApplicationVersion,

    /// For clients/streams: application icon. A binary blob containing PNG image data
    ApplicationIcon,

    /// For clients/streams: an XDG icon name for the application. E.g. "totem"
    ApplicationIconName,

    /// For clients/streams: application language if applicable, in standard POSIX format. E.g. "de_DE"
    ApplicationLanguage,

    /// For clients/streams on UNIX: application process PID, an integer formatted as string. E.g. "4711"
    ApplicationProcessId,

    /// For clients/streams: application process name. E.g. "totem"
    ApplicationProcessBinary,

    /// For clients/streams: application user name. E.g. "jonas"
    ApplicationProcessUser,

    /// For clients/streams: host name the application runs on. E.g. "omega"
    ApplicationProcessHost,

    /// For clients/streams: the D-Bus host id the application runs on. E.g. "543679e7b01393ed3e3e650047d78f6e"
    ApplicationProcessMachineId,

    /// For clients/streams: an id for the login session the application runs in. On Unix the value of $XDG_SESSION_ID. E.g. "5"
    ApplicationProcessSessionId,

    /// For devices: device string in the underlying audio layer's format. E.g. "surround51:0"
    DeviceString,

    /// For devices: API this device is access with. E.g. "alsa"
    DeviceApi,

    /// For devices: localized human readable device one-line description. E.g. "Foobar Industries USB Headset 2000+ Ultra"
    DeviceDescription,

    /// For devices: bus path to the device in the OS' format. E.g. "/sys/bus/pci/devices/0000:00:1f.2"
    DeviceBusPath,

    /// For devices: serial number if applicable. E.g. "4711-0815-1234"
    DeviceSerial,

    /// For devices: vendor ID if applicable. E.g. 1274
    DeviceVendorId,

    /// For devices: vendor name if applicable. E.g. "Foocorp Heavy Industries"
    DeviceVendorName,

    /// For devices: product ID if applicable. E.g. 4565
    DeviceProductId,

    /// For devices: product name if applicable. E.g. "SuperSpeakers 2000 Pro"
    DeviceProductName,

    /// For devices: device class. One of "sound", "modem", "monitor", "filter"
    DeviceClass,

    /// For devices: form factor if applicable. One of "internal", "speaker", "handset", "tv", "webcam", "microphone", "headset", "headphone", "hands-free", "car", "hifi", "computer", "portable"
    DeviceFormFactor,

    /// For devices: bus of the device if applicable. One of "isa", "pci", "usb", "firewire", "bluetooth"
    DeviceBus,

    /// For devices: icon for the device. A binary blob containing PNG image data
    DeviceIcon,

    /// For devices: an XDG icon name for the device. E.g. "sound-card-speakers-usb"
    DeviceIconName,

    /// For devices: access mode of the device if applicable. One of "mmap", "mmap_rewrite", "serial"
    DeviceAccessMode,

    /// For filter devices: master device id if applicable.
    DeviceMasterDevice,

    /// For devices: buffer size in bytes, integer formatted as string.
    DeviceBufferingBufferSize,

    /// For devices: fragment size in bytes, integer formatted as string.
    DeviceBufferingFragmentSize,

    /// For devices: profile identifier for the profile this devices is in. E.g. "analog-stereo", "analog-surround-40", "iec958-stereo", ...
    DeviceProfileName,

    /// For devices: intended use. A space separated list of roles (see PA_PROP_MEDIA_ROLE) this device is particularly well suited for, due to latency, quality or form factor. \since 0.9.16
    DeviceIntendedRoles,

    /// For devices: human readable one-line description of the profile this device is in. E.g. "Analog Stereo", ...
    DeviceProfileDescription,

    /// For modules: the author's name, formatted as UTF-8 string. E.g. "Lennart Poettering"
    ModuleAuthor,

    /// For modules: a human readable one-line description of the module's purpose formatted as UTF-8. E.g. "Frobnicate sounds with a flux compensator"
    ModuleDescription,

    /// For modules: a human readable usage description of the module's arguments formatted as UTF-8.
    ModuleUsage,

    /// For modules: a version string for the module. E.g. "0.9.15"
    ModuleVersion,

    /// For PCM formats: the sample format used as returned by pa_sample_format_to_string() \since 1.0
    FormatSampleFormat,

    /// For all formats: the sample rate (unsigned integer) \since 1.0
    FormatRate,

    /// For all formats: the number of channels (unsigned integer) \since 1.0
    FormatChannels,

    /// For PCM formats: the channel map of the stream as returned by pa_channel_map_snprint() \since 1.0
    FormatChannelMap,
}

impl Prop {
    /// Returns the property name to use in a property list.
    pub fn to_c_str(&self) -> &CStr {
        use self::Prop::*;

        match *self {
            MediaName => CStr::from_bytes_with_nul(b"media.name\0").unwrap(),
            MediaTitle => CStr::from_bytes_with_nul(b"media.title\0").unwrap(),
            MediaArtist => CStr::from_bytes_with_nul(b"media.artist\0").unwrap(),
            MediaCopyright => CStr::from_bytes_with_nul(b"media.copyright\0").unwrap(),
            MediaSoftware => CStr::from_bytes_with_nul(b"media.software\0").unwrap(),
            MediaLanguage => CStr::from_bytes_with_nul(b"media.language\0").unwrap(),
            MediaFilename => CStr::from_bytes_with_nul(b"media.filename\0").unwrap(),
            MediaIcon => CStr::from_bytes_with_nul(b"media.icon\0").unwrap(),
            MediaIconName => CStr::from_bytes_with_nul(b"media.icon_name\0").unwrap(),
            MediaRole => CStr::from_bytes_with_nul(b"media.role\0").unwrap(),
            FilterWant => CStr::from_bytes_with_nul(b"filter.want\0").unwrap(),
            FilterApply => CStr::from_bytes_with_nul(b"filter.apply\0").unwrap(),
            FilterSuppress => CStr::from_bytes_with_nul(b"filter.suppress\0").unwrap(),
            EventId => CStr::from_bytes_with_nul(b"event.id\0").unwrap(),
            EventDescription => CStr::from_bytes_with_nul(b"event.description\0").unwrap(),
            EventMouseX => CStr::from_bytes_with_nul(b"event.mouse.x\0").unwrap(),
            EventMouseY => CStr::from_bytes_with_nul(b"event.mouse.y\0").unwrap(),
            EventMouseHPos => CStr::from_bytes_with_nul(b"event.mouse.hpos\0").unwrap(),
            EventMouseVPos => CStr::from_bytes_with_nul(b"event.mouse.vpos\0").unwrap(),
            EventMouseButton => CStr::from_bytes_with_nul(b"event.mouse.button\0").unwrap(),
            WindowName => CStr::from_bytes_with_nul(b"window.name\0").unwrap(),
            WindowId => CStr::from_bytes_with_nul(b"window.id\0").unwrap(),
            WindowIcon => CStr::from_bytes_with_nul(b"window.icon\0").unwrap(),
            WindowIconName => CStr::from_bytes_with_nul(b"window.icon_name\0").unwrap(),
            WindowX => CStr::from_bytes_with_nul(b"window.x\0").unwrap(),
            WindowY => CStr::from_bytes_with_nul(b"window.y\0").unwrap(),
            WindowWidth => CStr::from_bytes_with_nul(b"window.width\0").unwrap(),
            WindowHeight => CStr::from_bytes_with_nul(b"window.height\0").unwrap(),
            WindowHPos => CStr::from_bytes_with_nul(b"window.hpos\0").unwrap(),
            WindowVPos => CStr::from_bytes_with_nul(b"window.vpos\0").unwrap(),
            WindowDesktop => CStr::from_bytes_with_nul(b"window.desktop\0").unwrap(),
            WindowX11Display => CStr::from_bytes_with_nul(b"window.x11.display\0").unwrap(),
            WindowX11Screen => CStr::from_bytes_with_nul(b"window.x11.screen\0").unwrap(),
            WindowX11Monitor => CStr::from_bytes_with_nul(b"window.x11.monitor\0").unwrap(),
            WindowX11Xid => CStr::from_bytes_with_nul(b"window.x11.xid\0").unwrap(),
            ApplicationName => CStr::from_bytes_with_nul(b"application.name\0").unwrap(),
            ApplicationId => CStr::from_bytes_with_nul(b"application.id\0").unwrap(),
            ApplicationVersion => CStr::from_bytes_with_nul(b"application.version\0").unwrap(),
            ApplicationIcon => CStr::from_bytes_with_nul(b"application.icon\0").unwrap(),
            ApplicationIconName => CStr::from_bytes_with_nul(b"application.icon_name\0").unwrap(),
            ApplicationLanguage => CStr::from_bytes_with_nul(b"application.language\0").unwrap(),
            ApplicationProcessId => CStr::from_bytes_with_nul(b"application.process.id\0").unwrap(),
            ApplicationProcessBinary => {
                CStr::from_bytes_with_nul(b"application.process.binary\0").unwrap()
            }
            ApplicationProcessUser => {
                CStr::from_bytes_with_nul(b"application.process.user\0").unwrap()
            }
            ApplicationProcessHost => {
                CStr::from_bytes_with_nul(b"application.process.host\0").unwrap()
            }
            ApplicationProcessMachineId => {
                CStr::from_bytes_with_nul(b"application.process.machine_id\0").unwrap()
            }
            ApplicationProcessSessionId => {
                CStr::from_bytes_with_nul(b"application.process.session_id\0").unwrap()
            }
            DeviceString => CStr::from_bytes_with_nul(b"device.string\0").unwrap(),
            DeviceApi => CStr::from_bytes_with_nul(b"device.api\0").unwrap(),
            DeviceDescription => CStr::from_bytes_with_nul(b"device.description\0").unwrap(),
            DeviceBusPath => CStr::from_bytes_with_nul(b"device.bus_path\0").unwrap(),
            DeviceSerial => CStr::from_bytes_with_nul(b"device.serial\0").unwrap(),
            DeviceVendorId => CStr::from_bytes_with_nul(b"device.vendor.id\0").unwrap(),
            DeviceVendorName => CStr::from_bytes_with_nul(b"device.vendor.name\0").unwrap(),
            DeviceProductId => CStr::from_bytes_with_nul(b"device.product.id\0").unwrap(),
            DeviceProductName => CStr::from_bytes_with_nul(b"device.product.name\0").unwrap(),
            DeviceClass => CStr::from_bytes_with_nul(b"device.class\0").unwrap(),
            DeviceFormFactor => CStr::from_bytes_with_nul(b"device.form_factor\0").unwrap(),
            DeviceBus => CStr::from_bytes_with_nul(b"device.bus\0").unwrap(),
            DeviceIcon => CStr::from_bytes_with_nul(b"device.icon\0").unwrap(),
            DeviceIconName => CStr::from_bytes_with_nul(b"device.icon_name\0").unwrap(),
            DeviceAccessMode => CStr::from_bytes_with_nul(b"device.access_mode\0").unwrap(),
            DeviceMasterDevice => CStr::from_bytes_with_nul(b"device.master_device\0").unwrap(),
            DeviceBufferingBufferSize => {
                CStr::from_bytes_with_nul(b"device.buffering.buffer_size\0").unwrap()
            }
            DeviceBufferingFragmentSize => {
                CStr::from_bytes_with_nul(b"device.buffering.fragment_size\0").unwrap()
            }
            DeviceProfileName => CStr::from_bytes_with_nul(b"device.profile.name\0").unwrap(),
            DeviceIntendedRoles => CStr::from_bytes_with_nul(b"device.intended_roles\0").unwrap(),
            DeviceProfileDescription => {
                CStr::from_bytes_with_nul(b"device.profile.description\0").unwrap()
            }
            ModuleAuthor => CStr::from_bytes_with_nul(b"module.author\0").unwrap(),
            ModuleDescription => CStr::from_bytes_with_nul(b"module.description\0").unwrap(),
            ModuleUsage => CStr::from_bytes_with_nul(b"module.usage\0").unwrap(),
            ModuleVersion => CStr::from_bytes_with_nul(b"module.version\0").unwrap(),
            FormatSampleFormat => CStr::from_bytes_with_nul(b"format.sample_format\0").unwrap(),
            FormatRate => CStr::from_bytes_with_nul(b"format.rate\0").unwrap(),
            FormatChannels => CStr::from_bytes_with_nul(b"format.channels\0").unwrap(),
            FormatChannelMap => CStr::from_bytes_with_nul(b"format.channel_map\0").unwrap(),
        }
    }

    /// Returns the property name as a string. Note that for compatibility with
    /// existing PulseAudio implementations, property keys must be
    /// null-terminated.
    pub fn to_str(&self) -> &str {
        // SAFETY: the strings above are all valid UTF-8.
        unsafe { std::str::from_utf8_unchecked(self.to_c_str().to_bytes()) }
    }
}

impl TagStructRead for Props {
    fn read(ts: &mut TagStructReader<'_>, _protocol_version: u16) -> Result<Self, ProtocolError> {
        ts.expect_tag(Tag::PropList)?;

        let mut props = Props::new();
        while let Some(key) = ts.read_string()? {
            if key.to_bytes().is_empty() {
                return Err(ProtocolError::Invalid("proplist key is empty".into()));
            }

            let len = ts.read_u32()?;
            if len > MAX_PROP_SIZE {
                return Err(ProtocolError::Invalid(format!(
                    "proplist value size {} exceeds hard limit of {} bytes",
                    len, MAX_PROP_SIZE
                )));
            }

            let value = ts.read_arbitrary()?;
            if len != value.len() as u32 {
                return Err(ProtocolError::Invalid(format!(
                    "proplist expected value size {} does not match actual size {}",
                    len,
                    value.len()
                )));
            }

            props.set_bytes(key, value.into_boxed_slice());
        }

        Ok(props)
    }
}

impl TagStructWrite for Props {
    fn write(
        &self,
        w: &mut TagStructWriter<'_>,
        _protocol_version: u16,
    ) -> Result<(), ProtocolError> {
        w.inner.write_u8(Tag::PropList as u8)?;

        for (k, v) in self.iter() {
            w.write_string(Some(k))?;
            w.write_u32(v.len() as u32)?;
            w.write_arbitrary(v)?;
        }

        w.write_null_string()?;
        Ok(())
    }
}

/// The mode of a [`Props`] update operation, used in various commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Primitive)]
pub enum PropsUpdateMode {
    /// Replace the entire property list with the new one.
    Set = 0,

    /// Merge the new property list with the current one without overwriting any values.
    Merge = 1,

    /// Merge the new property list with the current one, overwriting any values.
    Replace = 2,
}

#[cfg(test)]
mod tests {
    use crate::protocol::{test_util::test_serde_version, MAX_VERSION};

    use super::*;

    #[test]
    fn props_serde() -> anyhow::Result<()> {
        let mut props = Props::new();
        props.set_bytes(CString::new("foo")?, [1, 2, 3]);
        props.set(Prop::ApplicationName, CString::new("bar").unwrap());

        test_serde_version(&props, MAX_VERSION)?;
        Ok(())
    }
}
