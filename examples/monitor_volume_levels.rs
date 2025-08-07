// Monitor the volume levels of audio output devices in real-time.

use pulseaudio::protocol;
use std::{
    ffi::CString,
    fs,
    io::{BufReader, Read, ErrorKind},
    os::unix::net::UnixStream,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};
use std::error::Error;

/// Establishes a connection to the PulseAudio server using the default socket path from environment.
/// 
/// Returns a buffered UnixStream connection to PulseAudio or an error if connection fails.
pub fn connect_to_pulseaudio() -> Result<BufReader<UnixStream>, Box<dyn std::error::Error>> {
    let socket_path = pulseaudio::socket_path_from_env().ok_or("PulseAudio not available")?;
    let stream = UnixStream::connect(socket_path)?;
    Ok(BufReader::new(stream))
}


/// Authenticates with the PulseAudio server using the cookie from the default location.
/// 
/// # Arguments
/// * `sock` - Buffered UnixStream connection to PulseAudio
/// 
/// Returns the negotiated protocol version or an error if authentication fails.
pub fn authenticate(sock: &mut BufReader<UnixStream>) -> Result<u16, Box<dyn std::error::Error>> {
    let cookie = pulseaudio::cookie_path_from_env()
        .and_then(|path| fs::read(path).ok())
        .unwrap_or_default();

    let auth = protocol::AuthParams {
        version: protocol::MAX_VERSION,
        supports_shm: false,
        supports_memfd: false,
        cookie,
    };

    protocol::write_command_message(
        sock.get_mut(),
        0,
        protocol::Command::Auth(auth),
        protocol::MAX_VERSION,
    )?;

    let (_, auth_info) =
        protocol::read_reply_message::<protocol::AuthReply>(sock, protocol::MAX_VERSION)?;

    Ok(std::cmp::min(protocol::MAX_VERSION, auth_info.version))
}

/// Sets the client name for this PulseAudio connection to "pulseaudio-rs".
/// 
/// # Arguments
/// * `sock` - Buffered UnixStream connection to PulseAudio
/// * `protocol_version` - The negotiated protocol version from authentication
pub fn set_client_name(
    sock: &mut BufReader<UnixStream>,
    protocol_version: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut props = protocol::Props::new();
    props.set(protocol::Prop::ApplicationName, CString::new("pulseaudio-rs")?);

    protocol::write_command_message(
        sock.get_mut(),
        1,
        protocol::Command::SetClientName(props),
        protocol_version,
    )?;

    protocol::read_reply_message::<protocol::SetClientNameReply>(sock, protocol_version)?;
    Ok(())
}

/// Retrieves a list of all available audio sinks from PulseAudio.
/// 
/// # Arguments
/// * `sock` - Buffered UnixStream connection to PulseAudio
/// * `protocol_version` - The negotiated protocol version from authentication
/// 
/// Returns a vector of SinkInfo structures describing each available sink.
pub fn list_sinks(
    sock: &mut BufReader<UnixStream>,
    protocol_version: u16,
) -> Result<Vec<protocol::SinkInfo>, Box<dyn std::error::Error>> {
    protocol::write_command_message(
        sock.get_mut(),
        2,
        protocol::Command::GetSinkInfoList,
        protocol_version,
    )?;

    let (_, info_list) =
        protocol::read_reply_message::<protocol::SinkInfoList>(sock, protocol_version)?;

    Ok(info_list)
}

/// Creates a recording stream for monitoring a specific audio source.
/// 
/// # Arguments
/// * `sock` - Buffered UnixStream connection to PulseAudio
/// * `protocol_version` - The negotiated protocol version from authentication
/// * `monitor_source_name` - Name of the source to monitor
/// 
/// Returns the server's reply to the create stream request or an error if creation fails.
pub fn create_record_stream(
    sock: &mut BufReader<UnixStream>,
    protocol_version: u16,
    monitor_source_name: &str,
) -> Result<protocol::command::CreateRecordStreamReply, Box<dyn std::error::Error>> {
    let source_name = Some(CString::new(monitor_source_name)?);

    // Define low-latency buffer attributes
    let buffer_attr = protocol::serde::stream::BufferAttr {
        // A smaller max_length helps reduce latency
        max_length: 2048, // Small buffer size (2KB) for low latency
        
        // For record streams, fragment_size is important - it controls how much data 
        // is delivered at once. Smaller values reduce latency but increase CPU usage.
        fragment_size: 512, // Very small fragment size for minimum latency
        
        // Use default values for these fields (they're not as relevant for recording)
        target_length: u32::MAX,
        pre_buffering: u32::MAX,
        minimum_request_length: u32::MAX,
    };
    
    // Define stream flags that help with low latency
    let mut flags = protocol::serde::stream::StreamFlags::default();
    flags.adjust_latency = true;   // Request the server to adjust latency
    flags.early_requests = true;   // Request early data to prevent underruns
    
    let record_params = protocol::command::RecordStreamParams {
        sample_spec: protocol::serde::sample_spec::SampleSpec {
            format: protocol::serde::sample_spec::SampleFormat::S16Le,
            channels: 2,
            sample_rate: 44100,
        },
        channel_map: protocol::serde::channel_map::ChannelMap::default(),
        source_name,
        buffer_attr,
        flags,
        ..Default::default()
    };

    protocol::write_command_message(
        sock.get_mut(),
        3, // Command index for creating a recording stream
        protocol::command::Command::CreateRecordStream(record_params),
        protocol_version,
    )?;

    let (_, reply) = protocol::read_reply_message::<protocol::command::CreateRecordStreamReply>(
        sock,
        protocol_version,
    )?;

    Ok(reply)
}

/// Calculates the RMS (Root Mean Square) loudness of an audio buffer in decibels.
/// 
/// # Arguments
/// * `buffer` - Raw audio data as bytes (assumed to be 16-bit little-endian samples)
/// 
/// Returns the calculated loudness in decibels.
fn calculate_loudness(buffer: &[u8]) -> f64 {
    // Calculate the RMS loudness
    let samples = buffer
        .chunks_exact(2)
        .map(|chunk| i16::from_le_bytes([chunk[0], chunk[1]]) as f64);

    let sum_of_squares: f64 = samples.map(|s| s * s).sum();
    let rms = (sum_of_squares / buffer.len() as f64).sqrt();
    
    if rms < 0.45{ // if zero loadness
        return f64::NEG_INFINITY;
    }
    else{
        return 20.0 * rms.log10(); // Convert to decibels
    }
}

/// Reads and calculates the loudness from a recording stream without blocking.
/// 
/// # Arguments
/// * `sock` - Buffered UnixStream connection to PulseAudio
/// 
/// Returns the loudness in decibels or None if no data was available.
pub fn get_loudness_nonblocking(
    sock: &mut BufReader<UnixStream>,
) -> Result<Option<f64>, Box<dyn std::error::Error>> {
    // Use a small buffer size to reduce latency
    let buffer_size = 512; // Smaller buffer = faster processing
    let mut buffer = vec![0u8; buffer_size];

    // Set a read timeout to avoid waiting too long
    sock.get_mut().set_read_timeout(Some(Duration::from_millis(1)))?;
    
    // Try to read data from the stream with timeout
    match sock.get_mut().read(&mut buffer) {
        Ok(bytes_read) if bytes_read > 0 => {
            // Process the audio data to compute loudness
            let loudness = calculate_loudness(&buffer[..bytes_read]);
            Ok(Some(loudness))
        },
        Ok(_) => Ok(None), // No data available
        Err(ref e) if e.kind() == ErrorKind::WouldBlock || e.kind() == ErrorKind::TimedOut => {
            // Non-blocking socket would block or timeout - this is normal
            Ok(None)
        },
        Err(e) => Err(Box::new(e)), // Other error
    }
}

/// Sets up a non-blocking UnixStream socket.
pub fn set_nonblocking(stream: &UnixStream) -> Result<(), Box<dyn std::error::Error>> {
    use std::os::unix::io::AsRawFd;
    use libc::{fcntl, F_GETFL, F_SETFL, O_NONBLOCK};
    
    let fd = stream.as_raw_fd();
    
    // Get the current flags
    let flags = unsafe { fcntl(fd, F_GETFL, 0) };
    if flags < 0 {
        return Err("Failed to get socket flags".into());
    }
    
    // Set the non-blocking flag
    let result = unsafe { fcntl(fd, F_SETFL, flags | O_NONBLOCK) };
    if result < 0 {
        return Err("Failed to set socket to non-blocking mode".into());
    }
    
    Ok(())
}

/// Helper function to set up a connection to a specific sink monitor source
/// 
/// # Arguments
/// * `monitor_source_name` - Name of the monitor source to connect to
/// 
/// Returns a connected and authenticated socket ready for reading audio data
fn setup_sink_monitoring(monitor_source_name: &str) -> Result<BufReader<UnixStream>, Box<dyn std::error::Error>> {
    // Create a new connection
    let mut sock = connect_to_pulseaudio()?;
    let protocol_version = authenticate(&mut sock)?;
    set_client_name(&mut sock, protocol_version)?;
    
    // Create a recording stream
    create_record_stream(&mut sock, protocol_version, monitor_source_name)?;
    
    // Set socket to non-blocking mode
    set_nonblocking(sock.get_ref())?;
    
    Ok(sock)
}

/// Continuously monitors a sink connection in a separate thread and updates the
/// loudness value atomically.
/// 
/// # Arguments
/// * `monitor_source_name` - Name of the monitor source to connect to
/// * `loudness` - Shared atomic value to update with the latest loudness reading
pub fn monitor_sink_thread(
    monitor_source_name: String,
    loudness: Arc<Mutex<f64>>,
) -> Result<thread::JoinHandle<()>, Box<dyn std::error::Error>> {
    // Launch a thread that continuously monitors this sink
    let handle = thread::spawn(move || {
        // Set up a new connection inside the thread
        match setup_sink_monitoring(&monitor_source_name) {
            Ok(mut sock) => {
                // Loop forever, updating the loudness value
                loop {
                    // Try to get a new loudness reading
                    match get_loudness_nonblocking(&mut sock) {
                        Ok(Some(new_loudness)) => {
                            // Successfully got a reading, update the shared value
                            if let Ok(mut lock) = loudness.lock() {
                                *lock = new_loudness;
                            }
                            // Don't sleep at all when data is available - process as quickly as possible
                        },
                        Ok(None) => {
                            // No data available, sleep for a very short time to avoid spinning
                            // This is short enough to check frequently but not waste 100% CPU
                            thread::sleep(Duration::from_micros(100));
                        },
                        Err(_) => {
                            // Error reading from socket, sleep a bit longer
                            thread::sleep(Duration::from_millis(1));
                        }
                    }
                }
            },
            Err(_) => {
                // Failed to set up connection, thread exits
                return;
            }
        }
    });
    
    Ok(handle)
}



struct SinkConnection {
    name: String,
    monitor_source_name: String,
    loudness: Arc<Mutex<f64>>, // Changed to use thread-safe atomic access pattern
}

/// Main entry point for the audio monitoring application.
/// Sets up connections to all available PulseAudio sinks and continuously monitors
/// their audio levels, displaying them in a simple console interface.
pub fn main() -> Result<(), Box<dyn Error>> {
    // Set up initial connection to get the list of sinks
    let mut sock = connect_to_pulseaudio()?;
    let protocol_version = authenticate(&mut sock)?;
    set_client_name(&mut sock, protocol_version)?;

    let sinks = list_sinks(&mut sock, protocol_version)?;

    if sinks.is_empty() {
        println!("No sinks found.");
        return Ok(());
    }

    // Create sink connections and spawn monitoring threads
    let mut sink_connections = Vec::new();
    let mut thread_handles = Vec::new();

    for sink in sinks {
        // Extract the monitor source name
        let monitor_source_name = match &sink.monitor_source_name {
            Some(name) => name.to_str()?.to_string(),
            None => continue, // Skip sinks without monitor source
        };

        let sink_name = sink.name.to_str()?.to_string();
        println!("Setting up sink: {}", sink_name);
        println!("Using monitor source: {}", monitor_source_name);

        // Create shared loudness value
        let loudness = Arc::new(Mutex::new(f64::NEG_INFINITY));
        
        // Create a sink connection record
        sink_connections.push(SinkConnection {
            name: sink_name,
            monitor_source_name: monitor_source_name.clone(),
            loudness: Arc::clone(&loudness),
        });
        
        // Spawn a thread to monitor this sink
        let handle = monitor_sink_thread(monitor_source_name, loudness)?;
        thread_handles.push(handle);
    }

    // Give threads a moment to connect and start collecting data
    thread::sleep(Duration::from_millis(500));

    // Display loop - just reads the latest loudness values without blocking
    println!("\n{:-^60}", "Audio Levels Monitor");
    let refresh_rate = Duration::from_millis(8); // ~120 FPS for smoother display
    
    loop {
        // Update the display
        print!("\x1B[2J\x1B[1;1H"); // Clear screen
        println!("{:-^60}", "Audio Levels Monitor");
        println!("{:<30} | {:<20}", "Sink Name", "Loudness (dB)");
        println!("{:-^60}", "");

        // Display the latest loudness value for each sink
        for conn in &sink_connections {
            let loudness_value = match conn.loudness.lock() {
                Ok(guard) => *guard,
                Err(_) => f64::NEG_INFINITY, // In case the mutex is poisoned
            };
            println!("{:<30} | {:<20.2}", conn.name, loudness_value);
        }
        
        // Sleep a bit to avoid updating the display too frequently
        thread::sleep(refresh_rate);
    }
}
