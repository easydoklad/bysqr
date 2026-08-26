use std::{env, error::Error, fs, path::PathBuf};

use bysqr::qr::{
    create_invoice_svg_with_theme, InvoiceColor, InvoiceTheme, LogoLayout, LogoPosition,
};

const PAYLOAD: &str = include_str!(
    "../tests/fixtures/invoice/valid-interoperability-offline-official-current.payload.txt"
);

fn main() -> Result<(), Box<dyn Error>> {
    let destination = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/invoice-theme-gallery.html"));

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cards = String::new();
    for layout in LogoLayout::ALL {
        for position in LogoPosition::ALL {
            for color in InvoiceColor::ALL {
                let theme = InvoiceTheme::new(layout, position, color);
                let svg = String::from_utf8(create_invoice_svg_with_theme(PAYLOAD.trim(), theme))?;
                cards.push_str(&format!(
                    "<figure><div class=\"qr\">{svg}</div><figcaption>{layout:?} / {position:?} / {color:?}<br><code>{}</code></figcaption></figure>",
                    color.hex()
                ));
            }
        }
    }

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>INVOICE by square theme gallery</title>
<style>
  :root {{ color-scheme: light; font-family: system-ui, sans-serif; }}
  body {{ margin: 0; padding: 32px; background: #eceef1; color: #202124; }}
  h1 {{ margin: 0 0 8px; }}
  p {{ margin: 0 0 28px; color: #5f6062; }}
  main {{ display: grid; grid-template-columns: repeat(4, minmax(230px, 1fr)); gap: 20px; }}
  figure {{ margin: 0; padding: 16px; background: white; border-radius: 12px; box-shadow: 0 2px 10px #0001; }}
  .qr {{ display: grid; min-height: 290px; place-items: center; }}
  svg {{ display: block; width: 100%; height: auto; max-height: 340px; }}
  figcaption {{ margin-top: 12px; font-size: 14px; line-height: 1.5; }}
  code {{ color: #5f6062; }}
  @media (max-width: 1050px) {{ main {{ grid-template-columns: repeat(2, minmax(230px, 1fr)); }} }}
  @media (max-width: 560px) {{ body {{ padding: 16px; }} main {{ grid-template-columns: 1fr; }} }}
</style>
<h1>INVOICE by square theme gallery</h1>
<p>All 32 logo-manual presets: 2 layouts × 4 positions × 4 approved colors.</p>
<main>{cards}</main>
</html>
"#
    );
    fs::write(&destination, html)?;
    println!("{}", destination.display());
    Ok(())
}
