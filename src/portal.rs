use std::{
    cell::RefCell,
    num::NonZeroUsize,
    os::fd::{BorrowedFd, OwnedFd},
    path::PathBuf,
    rc::Rc,
};

use image::{ImageBuffer, Rgba, RgbaImage};
use nix::sys::mman::{MapFlags, MmapAdvise, ProtFlags};
use pipewire as pw;
use pw::{
    main_loop::MainLoopWeak,
    properties::properties,
    spa::{
        buffer::DataType,
        param::video::{VideoFormat, VideoInfoRaw},
    },
    stream::StreamFlags,
};

use crate::{
    clipboard,
    config::Config,
    error::{AppError, Result},
    notify,
};

struct UserData {
    format: VideoInfoRaw,
    image: Rc<RefCell<Option<RgbaImage>>>,
    ml_weak: MainLoopWeak,
}

pub fn capture(cfg: &Config) -> Result<PathBuf> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| AppError::Other(e.to_string()))?;

    let (node_id, fd) = rt.block_on(open_portal())?;

    // Use a true OS thread so PipeWire's main loop is fully isolated from the
    // tokio reactor — they both use signal handlers and thread-local state that
    // must not be shared.
    let image = std::thread::spawn(move || pipewire_capture(node_id, fd))
        .join()
        .map_err(|_| AppError::Other("PipeWire capture thread panicked".into()))??;

    let path = cfg.output_path();
    image.save(&path).map_err(AppError::from)?;
    clipboard::copy_to_clipboard(&path)?;
    notify::notify_success(&path);
    println!("{}", path.display());
    Ok(path)
}

async fn open_portal() -> Result<(u32, OwnedFd)> {
    use ashpd::desktop::{
        screencast::{
            CursorMode, OpenPipeWireRemoteOptions, Screencast, SelectSourcesOptions, SourceType,
        },
        PersistMode,
    };

    let proxy: Screencast = Screencast::new()
        .await
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?;

    let session = proxy
        .create_session(Default::default())
        .await
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?;

    proxy
        .select_sources(
            &session,
            SelectSourcesOptions::default()
                .set_cursor_mode(CursorMode::Hidden)
                .set_sources(SourceType::Monitor | SourceType::Window)
                .set_multiple(false)
                .set_restore_token(None)
                .set_persist_mode(PersistMode::DoNot),
        )
        .await
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?;

    let response = proxy
        .start(&session, None, Default::default())
        .await
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?
        .response()
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?;

    let stream = response
        .streams()
        .first()
        .ok_or(AppError::UserCancelled)?
        .clone();

    let fd = proxy
        .open_pipe_wire_remote(&session, OpenPipeWireRemoteOptions::default())
        .await
        .map_err(|e: ashpd::Error| AppError::Other(e.to_string()))?;

    Ok((stream.pipe_wire_node_id(), fd))
}

fn pipewire_capture(node_id: u32, fd: OwnedFd) -> Result<RgbaImage> {
    pw::init();

    let mainloop = pw::main_loop::MainLoopRc::new(None)
        .map_err(|e| AppError::Other(e.to_string()))?;
    let context = pw::context::ContextRc::new(&mainloop, None)
        .map_err(|e| AppError::Other(e.to_string()))?;
    let core = context
        .connect_fd_rc(fd, None)
        .map_err(|e| AppError::Other(e.to_string()))?;

    let image_cell: Rc<RefCell<Option<RgbaImage>>> = Rc::new(RefCell::new(None));
    let ml_weak = mainloop.downgrade();

    let user_data = UserData {
        format: Default::default(),
        image: image_cell.clone(),
        ml_weak,
    };

    let stream = pw::stream::StreamRc::new(
        core,
        "crop-hypr-capture",
        properties! {
            *pw::keys::MEDIA_TYPE => "Video",
            *pw::keys::MEDIA_CATEGORY => "Capture",
            *pw::keys::MEDIA_ROLE => "Screen",
        },
    )
    .map_err(|e| AppError::Other(e.to_string()))?;

    let _listener = stream
        .add_local_listener_with_user_data(user_data)
        .param_changed(|_, ud, id, param| {
            let Some(param) = param else { return };
            if id != pw::spa::param::ParamType::Format.as_raw() {
                return;
            }
            let Ok((mt, ms)) = pw::spa::param::format_utils::parse_format(param) else {
                return;
            };
            if mt != pw::spa::param::format::MediaType::Video
                || ms != pw::spa::param::format::MediaSubtype::Raw
            {
                return;
            }
            let _ = ud.format.parse(param);
        })
        .process(|stream, ud| {
            let Some(mut buf) = stream.dequeue_buffer() else {
                return;
            };
            let datas = buf.datas_mut();
            if datas.is_empty() {
                return;
            }

            // Safety: access the raw spa_data to check chunk pointer before
            // dereferencing it. A null chunk causes a panic inside chunk()
            // which, crossing the C FFI boundary, becomes SIGSEGV.
            let raw = datas[0].as_raw();
            if raw.chunk.is_null() {
                return;
            }
            let dt = datas[0].type_();
            let (chunk_offset, chunk_size, stride) = {
                let chunk = datas[0].chunk();
                (chunk.offset() as usize, chunk.size() as usize, chunk.stride())
            };
            if chunk_size == 0 {
                return;
            }

            let maybe_img = match dt {
                DataType::MemPtr => {
                    // Data pointer already mapped by PipeWire.
                    datas[0].data().and_then(|d: &mut [u8]| {
                        let end = (chunk_offset + chunk_size).min(d.len());
                        let frame = &d[chunk_offset.min(d.len())..end];
                        let w = ud.format.size().width;
                        let h = ud.format.size().height;
                        decode_frame(frame, w, h, stride as u32, ud.format.format())
                    })
                }
                DataType::MemFd => {
                    // Manually mmap the memfd. Must use spa_data.mapoffset
                    // (page-aligned offset into the fd) and spa_data.maxsize
                    // (total map size). Using chunk values here underestimates
                    // the map size and causes SIGSEGV on access.
                    let map_size = raw.maxsize as usize;
                    let map_offset = raw.mapoffset as i64;
                    let raw_fd = raw.fd as i32;

                    NonZeroUsize::new(map_size.max(1)).and_then(|len| {
                        let bfd = unsafe { BorrowedFd::borrow_raw(raw_fd) };
                        let ptr = unsafe {
                            nix::sys::mman::mmap(
                                None,
                                len,
                                ProtFlags::PROT_READ,
                                MapFlags::MAP_SHARED,
                                bfd,
                                map_offset,
                            )
                            .ok()?
                        };
                        let _ = unsafe {
                            nix::sys::mman::madvise(ptr, len.get(), MmapAdvise::MADV_SEQUENTIAL)
                        };
                        let slice = unsafe {
                            std::slice::from_raw_parts(ptr.as_ptr().cast::<u8>(), len.get())
                        };
                        let end = (chunk_offset + chunk_size).min(len.get());
                        let frame = &slice[chunk_offset.min(len.get())..end];
                        let w = ud.format.size().width;
                        let h = ud.format.size().height;
                        let img = decode_frame(frame, w, h, stride as u32, ud.format.format());
                        let _ = unsafe { nix::sys::mman::munmap(ptr, len.get()) };
                        img
                    })
                }
                _ => None,
            };

            if let Some(img) = maybe_img {
                *ud.image.borrow_mut() = Some(img);
            }
            if let Some(ml) = ud.ml_weak.upgrade() {
                ml.quit();
            }
        })
        .register()
        .map_err(|e| AppError::Other(e.to_string()))?;

    let obj = pw::spa::pod::object!(
        pw::spa::utils::SpaTypes::ObjectParamFormat,
        pw::spa::param::ParamType::EnumFormat,
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaType,
            Id,
            pw::spa::param::format::MediaType::Video
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::MediaSubtype,
            Id,
            pw::spa::param::format::MediaSubtype::Raw
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFormat,
            Choice,
            Enum,
            Id,
            VideoFormat::BGRA,
            VideoFormat::BGRA,
            VideoFormat::BGRx,
            VideoFormat::RGBA,
            VideoFormat::RGBx,
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoSize,
            Choice,
            Range,
            Rectangle,
            pw::spa::utils::Rectangle {
                width: 1920,
                height: 1080
            },
            pw::spa::utils::Rectangle {
                width: 1,
                height: 1
            },
            pw::spa::utils::Rectangle {
                width: 7680,
                height: 4320
            }
        ),
        pw::spa::pod::property!(
            pw::spa::param::format::FormatProperties::VideoFramerate,
            Choice,
            Range,
            Fraction,
            pw::spa::utils::Fraction { num: 1, denom: 1 },
            pw::spa::utils::Fraction { num: 0, denom: 1 },
            pw::spa::utils::Fraction {
                num: 1000,
                denom: 1
            }
        ),
    );

    let values: Vec<u8> = pw::spa::pod::serialize::PodSerializer::serialize(
        std::io::Cursor::new(Vec::new()),
        &pw::spa::pod::Value::Object(obj),
    )
    .map_err(|e| AppError::Other(e.to_string()))?
    .0
    .into_inner();

    let mut params = [pw::spa::pod::Pod::from_bytes(&values)
        .ok_or_else(|| AppError::Other("Failed to build PipeWire params pod".to_string()))?];

    stream
        .connect(
            pw::spa::utils::Direction::Input,
            Some(node_id),
            StreamFlags::AUTOCONNECT,
            &mut params,
        )
        .map_err(|e| AppError::Other(e.to_string()))?;

    mainloop.run();

    drop(_listener);

    image_cell
        .borrow_mut()
        .take()
        .ok_or_else(|| AppError::Other("Portal capture yielded no frame".to_string()))
}

fn decode_frame(data: &[u8], w: u32, h: u32, stride: u32, fmt: VideoFormat) -> Option<RgbaImage> {
    if w == 0 || h == 0 {
        return None;
    }
    let stride = stride as usize;
    let mut img: RgbaImage = ImageBuffer::new(w, h);
    for row in 0..h as usize {
        let row_start = row * stride;
        let row_end = row_start + (w as usize * 4);
        if row_end > data.len() {
            return None;
        }
        let src = &data[row_start..row_end];
        for col in 0..w as usize {
            let base = col * 4;
            let pixel = match fmt {
                // BGRA → RGBA
                VideoFormat::BGRA => Rgba([src[base + 2], src[base + 1], src[base], src[base + 3]]),
                // BGRx → RGBA (x → 255)
                VideoFormat::BGRx => {
                    Rgba([src[base + 2], src[base + 1], src[base], 255])
                }
                // RGBA → RGBA (pass-through)
                VideoFormat::RGBA => Rgba([src[base], src[base + 1], src[base + 2], src[base + 3]]),
                // RGBx → RGBA
                VideoFormat::RGBx => Rgba([src[base], src[base + 1], src[base + 2], 255]),
                _ => return None,
            };
            img.put_pixel(col as u32, row as u32, pixel);
        }
    }
    Some(img)
}
