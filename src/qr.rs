// overlay-add bgra
// command could also be used to display images

use qrcode::{Color, QrCode};
use tokio::io::{AsyncWrite, AsyncWriteExt, BufWriter};

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
