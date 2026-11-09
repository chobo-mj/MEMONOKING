use crate::*;
use std::{
    borrow::Cow,
    io::{Cursor, Write},
    path::{Path, PathBuf},
};
pub struct Asset {
    pub name: String,
    pub date: NaiveDate,
    pub bytes: Option<Vec<u8>>,
    pub thumbnail: Option<Vec<u8>>,
    pub temporary: Option<Vec<u8>>,
    pub path: Option<PathBuf>,
    pub texture: Option<egui::TextureHandle>,
}
impl Asset {
    fn displayed(&self) -> Option<&[u8]> {
        self.temporary.as_deref().or(self.bytes.as_deref())
    }
}
fn decode(bytes: &[u8]) -> Result<image::RgbaImage, String> {
    if bytes.len() > 64 * 1024 * 1024 {
        return Err("64 MB를 넘는 파일은 이 미리보기에서 열지 않습니다".into());
    }
    let mut reader = image::ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(256 * 1024 * 1024);
    reader.limits(limits);
    reader
        .decode()
        .map(|i| i.to_rgba8())
        .map_err(|e| e.to_string())
}
fn png(img: image::RgbaImage) -> Result<Vec<u8>, String> {
    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut out, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(out.into_inner())
}
fn draw_text(img: &mut image::RgbaImage, text: &str, x: u32, y: u32, size: f32) {
    static FONT: std::sync::OnceLock<Option<fontdue::Font>> = std::sync::OnceLock::new();
    let Some(font) = FONT.get_or_init(|| {
        std::fs::read("C:/Windows/Fonts/malgun.ttf")
            .ok()
            .and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
    }) else {
        return;
    };
    let mut left = x as i32;
    for ch in text.chars() {
        let (m, b) = font.rasterize(ch, size);
        for row in 0..m.height {
            for col in 0..m.width {
                let xx = left + m.xmin + col as i32;
                let yy = y as i32 + (size as i32 - m.height as i32 - m.ymin) + row as i32;
                if xx >= 0 && yy >= 0 && (xx as u32) < img.width() && (yy as u32) < img.height() {
                    let alpha = b[row * m.width + col] as f32 / 255.;
                    let p = img.get_pixel_mut(xx as u32, yy as u32);
                    for c in 0..3 {
                        p[c] = (p[c] as f32 * (1. - alpha) + 48. * alpha) as u8;
                    }
                }
            }
        }
        left += m.advance_width.ceil() as i32;
    }
}
fn table_png(rows: &[[&str; 3]]) -> Result<Vec<u8>, String> {
    let mut img = image::RgbaImage::from_pixel(
        660,
        rows.len() as u32 * 48 + 1,
        image::Rgba([255, 254, 252, 255]),
    );
    for (r, row) in rows.iter().enumerate() {
        for (c, text) in row.iter().enumerate() {
            draw_text(&mut img, text, 15 + c as u32 * 220, 10 + r as u32 * 48, 19.);
        }
    }
    for x in [0, 219, 439, 659] {
        for y in 0..img.height() {
            img.put_pixel(x, y, image::Rgba([210, 210, 205, 255]));
        }
    }
    for r in 0..=rows.len() {
        let y = (r as u32 * 48).min(img.height() - 1);
        for x in 0..img.width() {
            img.put_pixel(x, y, image::Rgba([210, 210, 205, 255]));
        }
    }
    png(img)
}
fn sample_asset(i: usize) -> Asset {
    let color = PALETTE[i];
    let mut img = image::RgbaImage::from_pixel(
        480,
        300,
        image::Rgba([color.r(), color.g(), color.b(), 255]),
    );
    draw_text(&mut img, ["기록", "아이디어", "참고 자료"][i], 35, 115, 38.);
    Asset {
        name: format!("예시 이미지 {}", i + 1),
        date: Local::now().date_naive() - chrono::Duration::days(i as i64),
        thumbnail: png(image::imageops::thumbnail(&img, 120, 75)).ok(),
        bytes: png(img).ok(),
        temporary: None,
        path: None,
        texture: None,
    }
}
pub fn parse_bookmarks(html: &str) -> Vec<(String, String)> {
    let doc = scraper::Html::parse_document(html);
    let selector = scraper::Selector::parse("a[href]").unwrap();
    let mut seen = std::collections::HashSet::new();
    doc.select(&selector)
        .filter_map(|a| {
            let href = a.value().attr("href")?.trim();
            if !(href.starts_with("https://") || href.starts_with("http://"))
                || href.chars().any(|c| c.is_control())
            {
                return None;
            }
            if !seen.insert(href.to_owned()) {
                return None;
            }
            Some((a.text().collect::<String>(), href.to_owned()))
        })
        .take(20000)
        .collect()
}
impl App {
    fn copy_image_bytes(&mut self, bytes: &[u8]) -> Result<(), String> {
        let img = decode(bytes)?;
        let mut c = arboard::Clipboard::new().map_err(|e| e.to_string())?;
        c.set_image(arboard::ImageData {
            width: img.width() as usize,
            height: img.height() as usize,
            bytes: Cow::Owned(img.into_raw()),
        })
        .map_err(|e| e.to_string())
    }
    pub fn real_table_gallery(&mut self, ui: &mut egui::Ui) {
        ui.heading("표 모아보기");
        ui.add(egui::TextEdit::singleline(&mut self.extra.table_search).hint_text("표 내용 검색"));
        let rows = [
            ["작업", "상태", "일정"],
            ["화면 설계", "진행 중", "오늘"],
            ["프로토타입 검토", "할 일", "내일"],
        ];
        if !self.extra.table_search.is_empty()
            && !rows
                .iter()
                .flatten()
                .any(|c| c.contains(&self.extra.table_search))
        {
            ui.label("검색 결과가 없습니다.");
            return;
        }
        egui::Grid::new("actual_table")
            .striped(true)
            .min_col_width(150.)
            .show(ui, |ui| {
                for r in rows {
                    for c in r {
                        ui.label(c);
                    }
                    ui.end_row();
                }
            });
        ui.horizontal(|ui| {
            if ui.button("표로 복사").clicked() {
                let text = rows
                    .iter()
                    .map(|r| r.join("\t"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let html = format!(
                    "<table>{}</table>",
                    rows.iter()
                        .map(|r| format!(
                            "<tr>{}</tr>",
                            r.iter()
                                .map(|c| format!("<td>{c}</td>"))
                                .collect::<String>()
                        ))
                        .collect::<String>()
                );
                self.toast = match arboard::Clipboard::new()
                    .and_then(|mut c| c.set_html(html, Some(text)))
                {
                    Ok(_) => "표를 HTML / 탭 구분 텍스트로 복사했습니다".into(),
                    Err(e) => format!("복사 실패: {e}"),
                };
            }
            if ui.button("이미지로 복사").clicked() {
                self.toast = match table_png(&rows).and_then(|b| self.copy_image_bytes(&b)) {
                    Ok(_) => "표 이미지를 클립보드에 복사했습니다".into(),
                    Err(e) => format!("복사 실패: {e}"),
                };
            }
        });
        ui.label(
            RichText::new("표 데이터는 예시입니다. 복사는 실제 클립보드에 반영됩니다.")
                .small()
                .color(MUTED),
        );
    }
    pub fn real_image_gallery(&mut self, ui: &mut egui::Ui) {
        if self.extra.media.is_empty() {
            self.extra.media = (0..3).map(sample_asset).collect();
        }
        ui.heading("이미지 모아보기");
        let resident: usize = self
            .extra
            .media
            .iter()
            .filter_map(|m| m.bytes.as_ref())
            .map(Vec::len)
            .sum();
        let temp: usize = self
            .extra
            .media
            .iter()
            .filter_map(|m| m.temporary.as_ref())
            .map(Vec::len)
            .sum();
        ui.label(format!(
            "{}개 · 본문 이미지 {:.1} KB · 임시 로딩 {:.1} KB",
            self.extra.media.len(),
            resident as f64 / 1024.,
            temp as f64 / 1024.
        ));
        ui.horizontal_wrapped(|ui| {
            self.date_filter_button(ui, 6, "시작 날짜", self.extra.media_from);
            self.date_filter_button(ui, 7, "끝 날짜", self.extra.media_to);
            if ui.button("필터 해제").clicked() {
                self.extra.media_from = None;
                self.extra.media_to = None;
            }
            if ui.button("일괄 저장하기").clicked() {
                if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                    let mut count = 0;
                    let mut errors = vec![];
                    for (i, m) in self.extra.media.iter_mut().enumerate() {
                        if self.extra.media_from.is_some_and(|a| m.date < a)
                            || self.extra.media_to.is_some_and(|b| m.date > b)
                        {
                            continue;
                        }
                        if let Some(bytes) = m.displayed() {
                            let path = dir.join(format!(
                                "memonoking-image-{}-{}.png",
                                i + 1,
                                Local::now().format("%Y%m%d-%H%M%S")
                            ));
                            match decode(bytes)
                                .and_then(png)
                                .and_then(|b| write_new(&path, &b))
                            {
                                Ok(()) => {
                                    m.path = Some(path);
                                    count += 1
                                }
                                Err(e) => errors.push(e),
                            }
                        }
                    }
                    self.toast = format!("{count}개 저장 · {}개 실패", errors.len());
                }
            }
        });
        let ids: Vec<_> = self
            .extra
            .media
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                self.extra.media_from.is_none_or(|a| m.date >= a)
                    && self.extra.media_to.is_none_or(|b| m.date <= b)
            })
            .map(|(i, _)| i)
            .collect();
        ui.horizontal_wrapped(|ui| {
            for i in ids {
                ui.push_id(i, |ui| {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.vertical(|ui| self.image_card(ui, i));
                    });
                });
            }
        });
        ui.label(RichText::new("저장 위치와 비운 상태는 앱 실행 중에만 유지됩니다. 프로그램 종료 후 복원은 저장 기능 연결 단계에서 지원합니다.").small().color(MUTED));
    }
    pub fn real_bookmark_gallery(&mut self, ui: &mut egui::Ui) {
        ui.heading("링크 · 즐겨찾기");
        if ui.button("즐겨찾기 가져오기…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("즐겨찾기 HTML", &["html", "htm"])
                .pick_file()
            {
                let result = std::fs::metadata(&path)
                    .map_err(|e| e.to_string())
                    .and_then(|m| {
                        if m.len() > 10 * 1024 * 1024 {
                            Err("10 MB를 넘는 HTML 파일은 가져오지 않습니다".into())
                        } else {
                            std::fs::read_to_string(path).map_err(|e| e.to_string())
                        }
                    });
                match result {
                    Ok(html) => {
                        let old = self.extra.bookmarks.len();
                        for b in parse_bookmarks(&html) {
                            if !self.extra.bookmarks.iter().any(|(_, u)| *u == b.1) {
                                self.extra.bookmarks.push(b);
                            }
                        }
                        self.toast =
                            format!("즐겨찾기 {}개 추가", self.extra.bookmarks.len() - old);
                    }
                    Err(e) => self.toast = format!("가져오기 실패: {e}"),
                }
            }
        }
        ui.label(RichText::new("Chrome / Edge의 UTF-8 HTML 내보내기 파일 · http/https만 허용 · 링크는 기본 브라우저로 열립니다.").small().color(MUTED));
        for (t, url) in &self.extra.bookmarks {
            ui.group(|ui| {
                ui.hyperlink_to(t, url);
                ui.label(RichText::new(url).small().color(MUTED));
            });
        }
    }
}
fn read_image(path: &Path) -> Result<Vec<u8>, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > 64 * 1024 * 1024 {
        return Err("64 MB 초과 이미지".into());
    }
    let b = std::fs::read(path).map_err(|e| e.to_string())?;
    decode(&b)?;
    Ok(b)
}
fn write_new(path: &Path, b: &[u8]) -> Result<(), String> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(b).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bookmarks_skip_script_urls_and_deduplicate() {
        assert_eq!(parse_bookmarks("<a href='javascript:alert(1)'>x</a><a href='https://example.com'>가 &amp; 나</a><a href='https://example.com'>duplicate</a>"),vec![("가 & 나".into(),"https://example.com".into())]);
    }
    #[test]
    fn sample_asset_is_valid_png() {
        let a = sample_asset(0);
        let img = decode(a.bytes.as_ref().unwrap()).unwrap();
        assert_eq!((img.width(), img.height()), (480, 300));
    }
    #[test]
    fn table_image_has_exact_rows() {
        let b = table_png(&[["가", "나", "다"], ["1", "2", "3"]]).unwrap();
        let i = decode(&b).unwrap();
        assert_eq!((i.width(), i.height()), (660, 97));
    }
}

#[cfg(test)]
mod filesystem_tests {
    use super::*;
    #[test]
    fn batch_output_never_overwrites_existing_file() {
        let p = std::env::temp_dir().join(format!(
            "memonoking-test-{}-{}.tmp",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        write_new(&p, b"original").unwrap();
        assert!(write_new(&p, b"replacement").is_err());
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
        std::fs::remove_file(p).unwrap();
    }
}
impl App {
    fn image_card(&mut self, ui: &mut egui::Ui, i: usize) {
        ui.set_width(235.);
        let thumb_only = self.extra.media[i].displayed().is_none();
        let m = &mut self.extra.media[i];
        if m.texture.is_none() {
            if let Some(bytes) = m.displayed().or(m.thumbnail.as_deref()) {
                if let Ok(img) = decode(bytes) {
                    let ci = egui::ColorImage::from_rgba_unmultiplied(
                        [img.width() as usize, img.height() as usize],
                        img.as_raw(),
                    );
                    m.texture = Some(ui.ctx().load_texture(
                        format!("image-{i}"),
                        ci,
                        egui::TextureOptions::LINEAR,
                    ));
                }
            }
        }
        let clicked = if let Some(t) = &m.texture {
            ui.add(
                egui::Image::new(t)
                    .fit_to_exact_size(Vec2::new(235., 147.))
                    .sense(egui::Sense::click()),
            )
            .on_hover_text(if thumb_only {
                "썸네일 · 클릭하여 백업 찾기"
            } else {
                "클릭하면 이미지가 복사됩니다"
            })
            .clicked()
        } else {
            ui.add_sized([235., 147.], egui::Button::new("백업 이미지 찾기"))
                .clicked()
        };
        if clicked {
            if thumb_only {
                self.extra.missing_image = Some(i);
            } else {
                let b = self.extra.media[i].displayed().map(ToOwned::to_owned);
                self.toast = match b
                    .ok_or("원본 없음".into())
                    .and_then(|b| self.copy_image_bytes(&b))
                {
                    Ok(()) => "이미지를 복사했습니다".into(),
                    Err(e) => e,
                };
            }
        }
        ui.strong(&self.extra.media[i].name);
        ui.label(format!(
            "{} · {}",
            self.extra.media[i].date,
            if thumb_only {
                "썸네일만 보관"
            } else {
                "원본 이미지"
            }
        ));
        if save_icon(ui).clicked() {
            if thumb_only {
                self.extra.missing_image = Some(i);
            } else if let Some(path) = rfd::FileDialog::new()
                .add_filter("PNG", &["png"])
                .set_file_name(format!("image-{}.png", i + 1))
                .save_file()
            {
                let result = self.extra.media[i]
                    .displayed()
                    .ok_or("이미지 없음".into())
                    .and_then(decode)
                    .and_then(png)
                    .and_then(|b| std::fs::write(&path, b).map_err(|e| e.to_string()));
                match result {
                    Ok(()) => {
                        self.extra.media[i].path = Some(path);
                        self.toast = "이미지를 저장했습니다".into()
                    }
                    Err(e) => self.toast = e,
                }
            }
        }
        if let Some(path) = self.extra.media[i].path.clone() {
            if self.extra.media[i].bytes.is_some()
                && ui.button("백업 확인 후 원본 용량 정리").clicked()
            {
                self.toast = match self.archive_asset(i, &path) {
                    Ok(()) => "원본을 정리하고 작은 썸네일을 남겼습니다".into(),
                    Err(e) => format!("정리 중단·원본 유지: {e}"),
                };
            }
        }
        if self.extra.media[i].temporary.is_some() && ui.button("백업 보기 닫기").clicked() {
            self.extra.media[i].temporary = None;
            self.extra.media[i].texture = None;
        }
    }
    fn archive_asset(&mut self, i: usize, path: &Path) -> Result<(), String> {
        let b = read_image(path)?;
        let external = decode(&b)?;
        let original = decode(self.extra.media[i].bytes.as_deref().ok_or("원본 없음")?)?;
        if original != external {
            return Err("백업 파일이 원본과 다릅니다".into());
        }
        let thumb = png(image::imageops::thumbnail(&original, 120, 75))?;
        let m = &mut self.extra.media[i];
        m.thumbnail = Some(thumb);
        m.bytes = None;
        m.temporary = None;
        m.texture = None;
        Ok(())
    }
    pub fn missing_image_dialog(&mut self, ctx: &egui::Context) {
        if let Some(i) = self.extra.missing_image {
            egui::Window::new("이미지 백업 찾기")
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label("용량 관리로 원본 이미지가 정리되었습니다. 백업 파일을 찾아주세요.");
                    if let Some(path) = self.extra.media[i].path.clone() {
                        ui.label(path.display().to_string());
                        if ui.button("기억한 백업에서 열기").clicked() {
                            match read_image(&path) {
                                Ok(b) => {
                                    self.extra.media[i].temporary = Some(b);
                                    self.extra.media[i].texture = None;
                                    self.extra.missing_image = None
                                }
                                Err(e) => self.toast = e,
                            }
                        }
                    }
                    if ui.button("백업 파일 찾기…").clicked() {
                        let mut dialog =
                            rfd::FileDialog::new().add_filter("이미지", &["png", "jpg", "jpeg"]);
                        if let Some(parent) =
                            self.extra.media[i].path.as_ref().and_then(|p| p.parent())
                        {
                            dialog = dialog.set_directory(parent);
                        }
                        if let Some(path) = dialog.pick_file() {
                            match read_image(&path) {
                                Ok(b) => {
                                    self.extra.media[i].temporary = Some(b);
                                    self.extra.media[i].path = Some(path);
                                    self.extra.media[i].texture = None;
                                    self.extra.missing_image = None
                                }
                                Err(e) => self.toast = e,
                            }
                        }
                    }
                    if ui.button("닫기").clicked() {
                        self.extra.missing_image = None;
                    }
                });
        }
    }
}
fn save_icon(ui: &mut egui::Ui) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(32., 28.), egui::Sense::click());
    r.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "다른 이름으로 저장")
    });
    let c = rect.center();
    let color = ui.visuals().text_color();
    let stroke = egui::Stroke::new(1.5_f32, color);
    ui.painter()
        .line_segment([c - Vec2::new(0., 7.), c + Vec2::new(0., 3.)], stroke);
    ui.painter()
        .line_segment([c + Vec2::new(-4., 0.), c + Vec2::new(0., 4.)], stroke);
    ui.painter()
        .line_segment([c + Vec2::new(4., 0.), c + Vec2::new(0., 4.)], stroke);
    ui.painter()
        .line_segment([c + Vec2::new(-7., 7.), c + Vec2::new(7., 7.)], stroke);
    r.on_hover_text("다른 이름으로 저장")
}
#[cfg(test)]
mod thumbnail_tests {
    use super::*;
    #[test]
    fn archive_keeps_thumbnail_and_rejects_wrong_backup() {
        let mut a = App::sample(Local::now().date_naive());
        a.extra.media = vec![sample_asset(0)];
        let p = std::env::temp_dir().join(format!(
            "memonoking-archive-{}-{}.png",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let original = a.extra.media[0].bytes.clone().unwrap();
        std::fs::write(&p, sample_asset(1).bytes.unwrap()).unwrap();
        assert!(a.archive_asset(0, &p).is_err());
        assert_eq!(a.extra.media[0].bytes.as_ref().unwrap(), &original);
        std::fs::write(&p, &original).unwrap();
        a.archive_asset(0, &p).unwrap();
        assert!(a.extra.media[0].bytes.is_none());
        let thumb = decode(a.extra.media[0].thumbnail.as_ref().unwrap()).unwrap();
        assert!(thumb.width() <= 120 && thumb.height() <= 75);
        assert_eq!(std::fs::read(&p).unwrap(), original);
        std::fs::remove_file(p).unwrap();
    }
}
