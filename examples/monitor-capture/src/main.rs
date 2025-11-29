use libobs_simple::output::simple::ObsContextSimpleExt;
#[cfg(target_os = "linux")]
use libobs_wrapper::logger::ObsLogger;
use libobs_wrapper::utils::StartupInfo;
use libobs_wrapper::{context::ObsContext, utils::ObsPath};

#[cfg(windows)]
use libobs_simple::sources::windows::{MonitorCaptureSourceBuilder, MonitorCaptureSourceUpdater};
#[cfg(windows)]
use libobs_wrapper::data::ObsObjectUpdater;
#[cfg(windows)]
use libobs_wrapper::sources::ObsSourceBuilder;
#[cfg(windows)]
use libobs_wrapper::utils::traits::ObsUpdatable;

#[cfg(target_os = "linux")]
use libobs_simple::sources::linux::LinuxGeneralScreenCapture;

#[cfg(target_os = "macos")]
use libobs_simple::sources::macos::ScreenCaptureSourceBuilder;
#[cfg(target_os = "macos")]
use libobs_wrapper::sources::ObsSourceBuilder;

#[cfg(any(target_os = "linux", target_os = "macos"))]
use std::io::{self, Write};

#[cfg(target_os = "linux")]
#[derive(Debug)]
pub struct NoLogger {}
#[cfg(target_os = "linux")]
impl ObsLogger for NoLogger {
    fn log(&mut self, _level: libobs_wrapper::enums::ObsLogLevel, _msg: String) {}
}

fn main() -> anyhow::Result<()> {
    // Start the OBS context
    let startup_info = StartupInfo::default();

    // FIXME This is not recommended in production. This is just for the purpose of this example.
    #[cfg(target_os = "linux")]
    let startup_info = startup_info.set_logger(Box::new(NoLogger {}));

    let mut context = ObsContext::new(startup_info)?;

    let mut scene = context.scene("main")?;

    // Platform-specific screen/monitor capture setup
    #[cfg(windows)]
    let monitors = MonitorCaptureSourceBuilder::get_monitors()?;

    #[cfg(windows)]
    let mut monitor_capture = context
        .source_builder::<MonitorCaptureSourceBuilder, _>("Monitor Capture")?
        .set_monitor(&monitors[0])
        .add_to_scene(&mut scene)?;

    #[cfg(target_os = "linux")]
    {
        // You could also read a restore token here from a file
        let screen_capture = LinuxGeneralScreenCapture::auto_detect(
            context.runtime().clone(),
            "Screen Capture",
            None,
        )
        .map_err(|e| anyhow::anyhow!("Failed to create screen capture: {}", e))?;

        println!(
            "Using {} capture method",
            screen_capture.capture_type_name()
        );

        screen_capture.add_to_scene(&mut scene)?;
    }

    #[cfg(target_os = "macos")]
    {
        // macOS uses ScreenCaptureKit via the screen_capture source
        let _screen_capture = context
            .source_builder::<ScreenCaptureSourceBuilder, _>("Screen Capture")?
            .set_display(0) // Main display
            .set_show_cursor(true)
            .add_to_scene(&mut scene)?;

        println!("Using macOS ScreenCaptureKit for screen capture");
    }

    // Common output and encoder setup
    scene.set_to_channel(0)?;

    // Set up output to ./recording.mp4
    let mut output = context
        .simple_output_builder("monitor-capture-output", ObsPath::new("record.mp4"))
        .build()?;

    output.start()?;

    #[cfg(windows)]
    {
        use std::thread;
        use std::time::Duration;

        println!("Recording for 5 seconds and switching monitor...");
        thread::sleep(Duration::from_secs(5));

        // Switching monitor
        monitor_capture
            .create_updater::<MonitorCaptureSourceUpdater>()?
            .set_monitor(&monitors[1])
            .update()?;

        println!("Recording for another 5 seconds...");
        thread::sleep(Duration::from_secs(5));
    }

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        print!("Recording... press Enter to stop.");
        io::stdout().flush()?;

        let mut buf = String::new();
        io::stdin().read_line(&mut buf)?;
    }

    // Stop recording
    output.stop()?;
    println!("Recording saved to recording.mp4");

    Ok(())
}
