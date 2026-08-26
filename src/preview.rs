use eframe::egui;
use eframe::egui::load::SizedTexture;
use eframe::egui::{Color32, ColorImage, Image, Margin, TextureOptions};

pub fn show_svg(code: Vec<u8>) -> Result<(), Box<dyn std::error::Error>> {
    let svg = crate::qr::map_svg(&code, crate::qr::CONTAINER_WIDTH as u32)?;
    let dimensions = [svg.width() as usize, svg.height() as usize];
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([svg.width() as f32, svg.height() as f32]),
        ..Default::default()
    };

    eframe::run_simple_native("QR Preview", options, move |ctx, _frame| {
        let bg = egui::containers::Frame {
            inner_margin: Margin::ZERO,
            outer_margin: Margin::ZERO,
            rounding: egui::Rounding::ZERO,
            shadow: eframe::epaint::Shadow::NONE,
            fill: Color32::WHITE,
            stroke: egui::Stroke::NONE,
        };

        egui::CentralPanel::default().frame(bg).show(ctx, |ui| {
            let color_image = ColorImage::from_rgba_unmultiplied(dimensions, svg.data());
            let texture = ctx.load_texture("qr.png", color_image, TextureOptions::default());
            let sized_texture = SizedTexture::from_handle(&texture);
            ui.add(Image::new(sized_texture))
        });
    })?;
    Ok(())
}
