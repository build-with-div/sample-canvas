//! Renders the three.js shell's scene (`fixtures/scene.json`) with the
//! port, on wgpu, and compares it with three.js 0.187.0's `WebGPURenderer`
//! in pinned Chromium (`fixtures/scene-chromium.json` and
//! `fixtures/scene-chromium/*.png`).
//!
//! Both sides render on SwiftShader. Chromium's WebGPU uses it through
//! Dawn. The port uses SwiftShader's Vulkan driver from Chrome for Testing
//! through wgpu, which `VK_DRIVER_FILES` (or `VK_ICD_FILENAMES`) points
//! at. SwiftShader in Chromium gives the same bytes from run to run
//! (docs/webgpu-oracle.md), so every pixel must match exactly.
//!
//! Without a SwiftShader adapter the test fails on CI (`CI` set) and is
//! skipped elsewhere, saying so.
//!
//!     cargo test -p three-render --test fidelity_scene -- --nocapture

use std::fs;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use serde_json::Value;
use three::math::{Quaternion, Vector3};
use three::{BufferGeometry, Index, MeshBasicMaterial, PerspectiveCamera, Scene};
use three_render::{RenderTarget, WebGpuRenderer};

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

fn read_json(name: &str) -> Value {
    serde_json::from_str(&fs::read_to_string(fixtures().join(name)).unwrap()).unwrap()
}

fn f64s(v: &Value) -> Vec<f64> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|x| x.as_f64().unwrap())
        .collect()
}

/// FNV-1a, 64-bit, as tools/three-render-reference/scene.js hashes.
fn fnv1a64(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        hash = (hash ^ u64::from(b)).wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn f32_bytes(values: &[f32]) -> Vec<u8> {
    values.iter().flat_map(|v| v.to_le_bytes()).collect()
}

fn geometry(spec: &Value) -> BufferGeometry {
    let a = f64s(&spec["args"]);
    match spec["type"].as_str().unwrap() {
        "box" => {
            BufferGeometry::box_geometry(a[0], a[1], a[2], a[3] as u32, a[4] as u32, a[5] as u32)
        }
        "sphere" => BufferGeometry::sphere_geometry(a[0], a[1] as u32, a[2] as u32),
        other => panic!("unknown geometry {other}"),
    }
}

/// The SwiftShader Vulkan device, or why there is none.
///
/// div-gap: W17 navigator.gpu. Div owns no wgpu instance or device to share
/// (gap 14), so the test creates its own, on the adapter it needs.
fn swiftshader() -> Result<(wgpu::Device, wgpu::Queue, wgpu::AdapterInfo), String> {
    let mut descriptor = wgpu::InstanceDescriptor::new_without_display_handle_from_env();
    descriptor.backends = wgpu::Backends::VULKAN;
    let instance = wgpu::Instance::new(descriptor);
    let adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    let names: Vec<String> = adapters.iter().map(|a| a.get_info().name).collect();
    let adapter = adapters
        .into_iter()
        .find(|a| a.get_info().name.contains("SwiftShader"))
        .ok_or_else(|| format!("no SwiftShader Vulkan adapter (found {names:?}); point VK_DRIVER_FILES at Chrome for Testing 147.0.7727.50's vk_swiftshader_icd.json"))?;
    let info = adapter.get_info();
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .map_err(|e| e.to_string())?;
    Ok((device, queue, info))
}

fn decode_png(path: &Path) -> (u32, u32, Vec<u8>) {
    let decoder = png::Decoder::new(std::io::Cursor::new(fs::read(path).unwrap()));
    let mut reader = decoder.read_info().unwrap();
    let mut pixels = vec![0; reader.output_buffer_size().unwrap()];
    let frame = reader.next_frame(&mut pixels).unwrap();
    assert_eq!(
        (frame.color_type, frame.bit_depth),
        (png::ColorType::Rgba, png::BitDepth::Eight)
    );
    pixels.truncate(frame.buffer_size());
    (frame.width, frame.height, pixels)
}

fn write_png(path: &Path, width: u32, height: u32, rgba: &[u8]) {
    let file = fs::File::create(path).unwrap();
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(rgba)
        .unwrap();
}

#[test]
fn scene_matches_three_in_chromium() {
    let spec = read_json("scene.json");
    let reference = read_json("scene-chromium.json");
    let browser = reference["chromium"].as_str().unwrap();

    // The scene, built as scene.js builds it.
    let mut scene = Scene::new();
    scene.background = Some(three::Color::from_hex(
        spec["background"].as_u64().unwrap() as u32
    ));
    let mut geometry_matched = 0;
    let mut geometry_total = 0;
    let mut failures = Vec::new();
    for object in spec["objects"].as_array().unwrap() {
        let name = object["name"].as_str().unwrap();
        let g = geometry(&object["geometry"]);
        let want = &reference["geometry"][name];
        let index_type = match g.index {
            Index::U16(_) => "Uint16Array",
            Index::U32(_) => "Uint32Array",
        };
        let got = [
            ("index", fnv1a64(&g.index.bytes())),
            ("indexType", index_type.to_string()),
            ("position", fnv1a64(&f32_bytes(&g.position))),
            ("normal", fnv1a64(&f32_bytes(&g.normal))),
            ("uv", fnv1a64(&f32_bytes(&g.uv))),
        ];
        for (field, value) in got {
            geometry_total += 1;
            if want[field].as_str() == Some(value.as_str()) {
                geometry_matched += 1;
            } else {
                failures.push(format!(
                    "{name} {field}: Div {value}, Chromium {}",
                    want[field]
                ));
            }
        }
        let p = f64s(&object["position"]);
        let r = f64s(&object["rotation"]);
        scene.add_mesh(
            Rc::new(g),
            MeshBasicMaterial::new(object["color"].as_u64().unwrap() as u32),
            Vector3::new(p[0], p[1], p[2]),
            [r[0], r[1], r[2]],
        );
    }
    println!("FIDELITY three-render geometry: {geometry_matched}/{geometry_total} attribute and index hashes match {browser}");

    let (device, queue, info) = match swiftshader() {
        Ok(found) => found,
        Err(why) if std::env::var_os("CI").is_some() => panic!("{why}"),
        Err(why) => {
            eprintln!("SKIPPED the pixel comparison: {why}");
            assert!(failures.is_empty(), "{}", failures.join("\n"));
            return;
        }
    };
    println!(
        "FIDELITY three-render adapter: {} ({})",
        info.name, info.driver
    );
    let renderer = WebGpuRenderer::new(device, queue);

    let out = Path::new(env!("CARGO_TARGET_TMPDIR")).join("three-render");
    let camera_spec = &spec["camera"];
    let (mut views_matched, mut pixels_matched, mut pixels_total) = (0, 0u64, 0u64);
    let views = spec["views"].as_array().unwrap();
    for view in views {
        let name = view["name"].as_str().unwrap();
        let (width, height) = (
            view["width"].as_u64().unwrap() as u32,
            view["height"].as_u64().unwrap() as u32,
        );
        let c = &view["camera"];
        let mut camera = PerspectiveCamera::new(
            camera_spec["fov"].as_f64().unwrap(),
            c["aspect"].as_f64().unwrap(),
            camera_spec["near"].as_f64().unwrap(),
            camera_spec["far"].as_f64().unwrap(),
        );
        let p = f64s(&c["position"]);
        camera.position = Vector3::new(p[0], p[1], p[2]);
        let q = f64s(&c["quaternion"]);
        camera.set_quaternion(Quaternion {
            x: q[0],
            y: q[1],
            z: q[2],
            w: q[3],
        });
        camera.zoom = c["zoom"].as_f64().unwrap();
        camera.update_projection_matrix();

        let target = RenderTarget::new(renderer.device(), width, height);
        renderer.render(&mut scene, &mut camera, &target);
        let got = renderer.read_render_target_pixels(&target);
        let (w, h, want) = decode_png(
            &fixtures()
                .join("scene-chromium")
                .join(format!("{name}.png")),
        );
        assert_eq!((w, h), (width, height), "{name}: reference size");

        let (mut differing, mut max) = (0u64, 0u8);
        for (a, b) in got.chunks_exact(4).zip(want.chunks_exact(4)) {
            let d = a.iter().zip(b).map(|(x, y)| x.abs_diff(*y)).max().unwrap();
            if d > 0 {
                differing += 1;
                max = max.max(d);
            }
        }
        let total = u64::from(width) * u64::from(height);
        pixels_total += total;
        pixels_matched += total - differing;
        if differing == 0 {
            views_matched += 1;
        } else {
            fs::create_dir_all(&out).unwrap();
            let path = out.join(format!("{name}.png"));
            write_png(&path, width, height, &got);
            failures.push(format!("{name}: {differing} of {total} pixels differ, max channel difference {max} (Div's render: {})", path.display()));
        }
    }
    println!(
        "FIDELITY three-render views: {views_matched}/{} byte-identical to {browser}",
        views.len()
    );
    println!("FIDELITY three-render pixels: {pixels_matched}/{pixels_total} match");
    for failure in &failures {
        println!("DIFF {failure}");
    }
    assert!(
        failures.is_empty(),
        "{} differences from three.js in Chromium",
        failures.len()
    );
}
