// reference
// https://github.com/microsoft/windows-rs/blob/d40a51812f0a943f1c2124948ef4436d9276dbf4/crates/samples/direct2d/src/main.rs
use windows::{
    core::*, Win32::Foundation::*, Win32::Graphics::Direct2D::Common::*,
    Win32::Graphics::Direct2D::*, Win32::Graphics::Direct3D::*, Win32::Graphics::Direct3D11::*,
    Win32::Graphics::Dxgi::Common::*, Win32::Graphics::Dxgi::*, Win32::System::LibraryLoader::*,
    Win32::UI::WindowsAndMessaging::*,
};
pub struct Window {
    handle: HWND,
}

fn main() -> Result<()> {
    let mut window = Window::new()?;
    start_keyboard_hook(window.handle)?;
    window.run()
}

impl Window {
    pub fn new() -> Result<Self> {
        Ok(Self { handle: HWND(0) })
    }

    pub fn run(&mut self) -> Result<()> {
        unsafe {
            let instance = GetModuleHandleA(None)?;
            debug_assert!(instance.0 != 0);
            dbg!(instance);

            let class_name = PCSTR(b"Example Class Name\0".as_ptr());

            let wc = WNDCLASSA {
                hInstance: instance,
                lpszClassName: class_name,
                lpfnWndProc: Some(Self::wndproc),
                ..Default::default()
            };

            let atom = RegisterClassA(&wc);
            debug_assert!(atom != 0);

            let handle = CreateWindowExA(
                Default::default(),
                class_name,
                PCSTR(b"example\0".as_ptr()),
                WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_VISIBLE,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                200,
                200,
                None,
                None,
                instance,
                self as *mut _ as _,
            );

            debug_assert!(handle.0 != 0);
            debug_assert!(handle == self.handle);
            dbg!(handle);

            let join_handle = std::thread::spawn(move || start_render_loop(handle));

            let mut message = MSG::default();
            let hwnd_default = HWND::default();
            let mut is_thread_finished = false;
            loop {
                while PeekMessageA(&mut message, hwnd_default, 0, 0, PM_REMOVE).as_bool() {
                    if message.message == WM_QUIT {
                        if join_handle.is_finished() {
                            let _ = dbg!(join_handle.join().unwrap());
                        }
                        return Ok(());
                    }
                    DispatchMessageA(&message);
                }
                if join_handle.is_finished() && !is_thread_finished {
                    PostQuitMessage(0);
                    is_thread_finished = true;
                }
            }
        }
    }

    extern "system" fn wndproc(
        window: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        unsafe {
            if message == WM_NCCREATE {
                let cs = lparam.0 as *const CREATESTRUCTA;
                let this = (*cs).lpCreateParams as *mut Self;
                (*this).handle = window;

                SetWindowLongPtrA(window, GWLP_USERDATA, this as isize);
            } else {
                let this = GetWindowLongPtrA(window, GWLP_USERDATA) as *mut Self;

                if !this.is_null() {
                    return (*this).message_handler(message, wparam, lparam);
                }
            }

            DefWindowProcA(window, message, wparam, lparam)
        }
    }

    fn message_handler(&mut self, message: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        unsafe {
            match message {
                WM_DESTROY => {
                    PostQuitMessage(0);
                    LRESULT(0)
                }
                _ => DefWindowProcA(self.handle, message, wparam, lparam),
            }
        }
    }
}

fn start_render_loop(handle: HWND) -> Result<()> {
    let factory = create_factory()?;

    let mut dpix = 0.0;
    let mut dpiy = 0.0;
    unsafe { factory.GetDesktopDpi(&mut dpix, &mut dpiy) };
    dbg!(dpix, dpiy);

    let device = create_device()?;
    let target = create_render_target(&factory, &device)?;
    unsafe { target.SetDpi(dpix, dpiy) };

    let swapchain = create_swapchain(&device, handle)?;
    create_swapchain_bitmap(&swapchain, &target)?;

    loop {
        unsafe {
            target.BeginDraw();
            draw(&target);
            target.EndDraw(std::ptr::null_mut(), std::ptr::null_mut())?;
            if let Err(e) = swapchain.Present(1, 0).ok() {
                dbg!(e);
                return Ok(());
            }
        };
    }
}

static mut R: f32 = 0.0;
static mut G: f32 = 0.0; // updated by low_level_keyboard_proc
unsafe fn draw(target: &ID2D1DeviceContext) {
    target.Clear(&D2D1_COLOR_F {
        r: R,
        g: G,
        b: 1.0,
        a: 1.0,
    });
    R = (R + 1.0 / 255.0) % 1.0;
}

fn create_factory() -> Result<ID2D1Factory1> {
    let mut options = D2D1_FACTORY_OPTIONS::default();

    if cfg!(debug_assertions) {
        options.debugLevel = D2D1_DEBUG_LEVEL_INFORMATION;
    }

    unsafe { D2D1CreateFactory(D2D1_FACTORY_TYPE_SINGLE_THREADED, &options) }
}

fn create_device() -> Result<ID3D11Device> {
    let mut result = create_device_with_type(D3D_DRIVER_TYPE_HARDWARE);

    if let Err(err) = &result {
        if err.code() == DXGI_ERROR_UNSUPPORTED {
            result = create_device_with_type(D3D_DRIVER_TYPE_WARP);
        }
    }

    result
}

fn create_device_with_type(drive_type: D3D_DRIVER_TYPE) -> Result<ID3D11Device> {
    let mut flags = D3D11_CREATE_DEVICE_BGRA_SUPPORT;

    if cfg!(debug_assertions) {
        flags |= D3D11_CREATE_DEVICE_DEBUG;
    }

    let mut device = None;

    unsafe {
        D3D11CreateDevice(
            None,
            drive_type,
            HINSTANCE::default(),
            flags,
            &[],
            D3D11_SDK_VERSION,
            &mut device,
            std::ptr::null_mut(),
            &mut None,
        )
        .map(|()| device.unwrap())
    }
}

fn create_render_target(
    factory: &ID2D1Factory1,
    device: &ID3D11Device,
) -> Result<ID2D1DeviceContext> {
    unsafe {
        let d2device = factory.CreateDevice(&device.cast::<IDXGIDevice>()?)?;
        let target = d2device.CreateDeviceContext(D2D1_DEVICE_CONTEXT_OPTIONS_NONE)?;

        target.SetUnitMode(D2D1_UNIT_MODE_DIPS);

        Ok(target)
    }
}

fn create_swapchain(device: &ID3D11Device, window: HWND) -> Result<IDXGISwapChain1> {
    let factory = get_dxgi_factory(device)?;

    let props = DXGI_SWAP_CHAIN_DESC1 {
        Format: DXGI_FORMAT_B8G8R8A8_UNORM,
        SampleDesc: DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        },
        BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
        BufferCount: 2,
        SwapEffect: DXGI_SWAP_EFFECT_FLIP_SEQUENTIAL,
        ..Default::default()
    };

    unsafe { factory.CreateSwapChainForHwnd(device, window, &props, std::ptr::null(), None) }
}

fn get_dxgi_factory(device: &ID3D11Device) -> Result<IDXGIFactory2> {
    let dxdevice = device.cast::<IDXGIDevice>()?;
    unsafe { dxdevice.GetAdapter()?.GetParent() }
}

fn create_swapchain_bitmap(swapchain: &IDXGISwapChain1, target: &ID2D1DeviceContext) -> Result<()> {
    let surface: IDXGISurface = unsafe { swapchain.GetBuffer(0)? };

    let props = D2D1_BITMAP_PROPERTIES1 {
        pixelFormat: D2D1_PIXEL_FORMAT {
            format: DXGI_FORMAT_B8G8R8A8_UNORM,
            alphaMode: D2D1_ALPHA_MODE_IGNORE,
        },
        dpiX: 96.0,
        dpiY: 96.0,
        bitmapOptions: D2D1_BITMAP_OPTIONS_TARGET | D2D1_BITMAP_OPTIONS_CANNOT_DRAW,
        colorContext: None,
    };

    unsafe {
        let bitmap = target.CreateBitmapFromDxgiSurface(&surface, &props)?;
        target.SetTarget(&bitmap);
    };

    Ok(())
}

static mut HOOK: HHOOK = HHOOK(0);

fn start_keyboard_hook(hwnd: HWND) -> Result<()> {
    unsafe {
        let hmod = GetWindowLongPtrA(hwnd, GWL_HINSTANCE);
        HOOK = SetWindowsHookExA(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            HINSTANCE(hmod),
            0,
        )?;
        dbg!(hmod, HOOK);
    };
    Ok(())
}

extern "system" fn low_level_keyboard_proc(ncode: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe {
        if ncode == HC_ACTION as i32 {
            match wparam.0 as u32 {
                WM_KEYDOWN => {
                    let kbd_ll_hook_struct: &KBDLLHOOKSTRUCT = std::mem::transmute(lparam);
                    let &KBDLLHOOKSTRUCT {
                        vkCode: vk_code,
                        scanCode: scan_code,
                        time,
                        ..
                    } = kbd_ll_hook_struct;
                    eprintln!("T:{vk_code}\t{vk_code:x}\t{scan_code}\t{scan_code:x}\t{time}",);

                    G = (G + vk_code as f32 / 255.0) % 1.0;
                }
                WM_KEYUP => {
                    let kbd_ll_hook_struct: &KBDLLHOOKSTRUCT = std::mem::transmute(lparam);
                    let &KBDLLHOOKSTRUCT {
                        vkCode: vk_code,
                        scanCode: scan_code,
                        time,
                        ..
                    } = kbd_ll_hook_struct;
                    eprintln!("F:{vk_code}\t{vk_code:x}\t{scan_code}\t{scan_code:x}\t{time}",);
                }
                _ => (),
            }
        }
        CallNextHookEx(HOOK, ncode, wparam, lparam)
    }
}
