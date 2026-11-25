use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::Duration;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::Graphics::Capture::GraphicsCaptureItem;
use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::frame::{Frame, ImageFormat};
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::settings::{
    CaptureItemTypes, ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings, TryIntoCaptureItemWithType,
};
use windows_capture::window::Window as NativeWindow;

/// Bind the capture item while GPUI still owns the live window. A delayed
/// background capture must never look up a recycled HWND or the foreground window.
pub(crate) struct CaptureTarget {
    item: GraphicsCaptureItem,
    window: NativeWindow,
}

pub(crate) fn capture_target(window: &gpui::Window) -> Result<CaptureTarget, String> {
    let handle = HasWindowHandle::window_handle(window).map_err(|e| e.to_string())?;
    let RawWindowHandle::Win32(native) = handle.as_raw() else {
        return Err("Expected a Windows application window".into());
    };
    let window = NativeWindow::from_raw_hwnd(native.hwnd.get() as *mut _);
    if window.process_id().ok() != Some(std::process::id()) {
        return Err("The window is no longer owned by this application".into());
    }
    let (item, _) = window.try_into_capture_item().map_err(|e| e.to_string())?;
    Ok(CaptureTarget { item, window })
}

impl TryIntoCaptureItemWithType for CaptureTarget {
    fn try_into_capture_item(
        self,
    ) -> windows::core::Result<(GraphicsCaptureItem, CaptureItemTypes)> {
        Ok((self.item, CaptureItemTypes::Window(self.window)))
    }
}

type FrameResult = Result<(u32, u32), String>;

struct SingleFrame {
    path: PathBuf,
    result: mpsc::Sender<FrameResult>,
}

impl GraphicsCaptureApiHandler for SingleFrame {
    type Flags = Self;
    type Error = String;

    fn new(context: Context<Self::Flags>) -> Result<Self, Self::Error> {
        Ok(context.flags)
    }

    fn on_frame_arrived(
        &mut self,
        frame: &mut Frame,
        control: InternalCaptureControl,
    ) -> Result<(), Self::Error> {
        let result = frame
            .save_as_image(&self.path, ImageFormat::Png)
            .map(|()| (frame.width(), frame.height()))
            .map_err(|e| format!("Could not encode the captured window: {e}"));
        control.stop();
        let _ = self.result.send(result);
        Ok(())
    }

    fn on_closed(&mut self) -> Result<(), Self::Error> {
        let _ = self.result.send(Err("Window closed before capture".into()));
        Ok(())
    }
}

impl CaptureTarget {
    pub(crate) fn save(self, path: &Path) -> FrameResult {
        let (result, receiver) = mpsc::channel();
        let settings = Settings::new(
            self,
            CursorCaptureSettings::Default,
            DrawBorderSettings::Default,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Rgba8,
            SingleFrame {
                path: path.to_owned(),
                result,
            },
        );
        let capture = SingleFrame::start_free_threaded(settings)
            .map_err(|e| format!("Windows Graphics Capture could not start: {e}"))?;
        match receiver.recv_timeout(Duration::from_secs(3)) {
            Ok(result) => {
                capture
                    .wait()
                    .map_err(|e| format!("Window capture failed: {e}"))?;
                result
            }
            Err(_) => {
                capture
                    .stop()
                    .map_err(|e| format!("Could not stop window capture: {e}"))?;
                Err("Windows did not supply a window frame in time".into())
            }
        }
    }
}
