use std::{env, error::Error, fs, path::PathBuf};

use bysqr::qr::{
    create_invoice_svg_with_theme, create_pay_svg_with_theme, LogoColor, LogoLayout, LogoPosition,
    LogoTheme,
};

const PAY_PAYLOAD: &str = "000620000OTQ8GD9P3146TKTM0EOR8GS6MCNQU8Q6DBV3A4QK1JTORU2PBR6CBS3SL85PJRVSIR8RE49VEGTF5JRTM45DTL9US038PVH5GCIC1483NLI0MGU6FF0KB8";
const INVOICE_PAYLOAD: &str = include_str!(
    "../tests/fixtures/invoice/valid-interoperability-offline-official-current.payload.txt"
);

fn main() -> Result<(), Box<dyn Error>> {
    let destination = env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("target/theme-preview.html"));

    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut cards = String::new();
    for layout in LogoLayout::ALL {
        for position in LogoPosition::ALL {
            for color in LogoColor::ALL {
                let theme = LogoTheme::new(layout, position, color);
                let pay = String::from_utf8(create_pay_svg_with_theme(PAY_PAYLOAD, theme)?)?;
                let invoice = String::from_utf8(create_invoice_svg_with_theme(
                    INVOICE_PAYLOAD.trim(),
                    theme,
                )?)?;
                cards.push_str(&format!(
                    "<figure><div class=\"pair\"><div><b>PAY</b>{pay}</div><div><b>INVOICE</b>{invoice}</div></div><figcaption>{layout:?} / {position:?} / {color:?}<br><code>PAY {} · INVOICE {}</code></figcaption></figure>",
                    color.pay_hex(),
                    color.invoice_hex(),
                ));
            }
        }
    }

    let html = format!(
        r#"<!doctype html>
<html lang="en">
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>by-square theme preview</title>
<style>
  :root {{ color-scheme: light; font-family: system-ui, sans-serif; }}
  body {{ margin: 0; padding: 32px; background: #eceef1; color: #202124; }}
  h1 {{ margin: 0 0 8px; }}
  p {{ margin: 0 0 28px; color: #5f6062; }}
  main {{ display: grid; grid-template-columns: repeat(2, minmax(420px, 1fr)); gap: 20px; }}
  figure {{ margin: 0; padding: 16px; background: white; border-radius: 12px; box-shadow: 0 2px 10px #0001; }}
  .pair {{ display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 12px; }}
  .pair div {{ display: grid; min-height: 300px; place-items: center; }}
  .pair b {{ align-self: end; font-size: 12px; letter-spacing: .08em; }}
  svg {{ display: block; width: 100%; height: auto; max-height: 300px; }}
  figcaption {{ margin-top: 12px; font-size: 14px; line-height: 1.5; }}
  code {{ color: #5f6062; }}
  @media (max-width: 960px) {{ main {{ grid-template-columns: 1fr; }} }}
  @media (max-width: 560px) {{ body {{ padding: 16px; }} .pair {{ grid-template-columns: 1fr; }} }}
</style>
<h1>PAY and INVOICE by-square theme preview</h1>
<p>All 32 logo-manual presets. Light and dark use the family palette; gray and black are shared.</p>
<main>{cards}</main>
</html>
"#
    );
    fs::write(&destination, html)?;
    println!("{}", destination.display());
    Ok(())
}
