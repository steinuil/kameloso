use std::path::Path;

use qrcode::{Color, QrCode};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

use crate::mpv;

const WHITE: [u8; 4] = [0xFF, 0xFF, 0xFF, 0];
const BLACK: [u8; 4] = [0, 0, 0, 0];

pub async fn write_bgra<W: AsyncWrite + std::marker::Unpin>(
    code: &QrCode,
    magnification: u8,
    out: &mut BufWriter<W>,
) -> tokio::io::Result<()> {
    for _ in 0..magnification {
        for _ in 0..code.width() + 2 {
            for _ in 0..magnification {
                out.write_all(&WHITE).await?;
            }
        }
    }

    for line in code.to_colors().chunks(code.width()) {
        for _ in 0..magnification {
            for _ in 0..magnification {
                out.write_all(&WHITE).await?;
            }
            for color in line {
                match color {
                    Color::Light => {
                        for _ in 0..magnification {
                            out.write_all(&WHITE).await?;
                        }
                    }
                    Color::Dark => {
                        for _ in 0..magnification {
                            out.write_all(&BLACK).await?;
                        }
                    }
                };
            }
            for _ in 0..magnification {
                out.write_all(&WHITE).await?;
            }
        }
    }

    for _ in 0..magnification {
        for _ in 0..code.width() + 2 {
            for _ in 0..magnification {
                out.write_all(&WHITE).await?;
            }
        }
    }

    out.flush().await?;

    Ok(())
}

pub async fn generate_qr_code(
    url: &str,
    qr_code_path: &Path,
    magnification: u8,
) -> tokio::io::Result<u32> {
    let qr_code = qrcode::QrCode::new(url.as_bytes()).unwrap();

    let f = tokio::fs::File::create(&qr_code_path).await?;
    let mut out = tokio::io::BufWriter::new(f);

    write_bgra(&qr_code, magnification, &mut out).await?;

    Ok(qr_code.width() as u32)
}

#[derive(Debug, Clone)]
pub struct QrCodeParams {
    pub path: String,
    pub width: u32,
    pub magnification: u8,
    pub active: bool,
}

const QR_CODE_OVERLAY_ID: u8 = 3;

pub async fn add_qr_code_overlay(
    mpv: &mpv::Client,
    params: &QrCodeParams,
) -> Result<(), mpv::Error> {
    mpv.overlay_add(&mpv::OverlayAddOptions {
        id: QR_CODE_OVERLAY_ID,
        x: 20,
        y: 20,
        file: params.path.clone(),
        w: (params.width + 2) * params.magnification as u32,
        h: (params.width + 2) * params.magnification as u32,
        offset: 0,
    })
    .await
}

pub async fn remove_qr_code_overlay(mpv: &mpv::Client) -> Result<(), mpv::Error> {
    let v = mpv.overlay_remove(QR_CODE_OVERLAY_ID).await?;
    dbg!(v);
    Ok(())
}
