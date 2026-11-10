//! Collections projected from the live memo documents, rather than sample flags.
use crate::composer::{Block, Picture, Table};
use crate::*;
use std::borrow::Cow;
use std::hash::{Hash, Hasher};
use std::io::{Cursor, Write};
use std::path::{Path, PathBuf};

type Source = (usize, usize, usize);

impl App {
    fn content_sources(&self, images: bool) -> Vec<Source> {
        let mut sources = vec![];
        for (i, n) in self.data.notes.iter().enumerate() {
            if !self.visible(i) || n.locked || n.purged {
                continue;
            }
            for (tab, document) in n.document.tabs.iter().enumerate() {
                for (block, item) in document.blocks.iter().enumerate() {
                    // A picture whose placeholder was deleted from the text
                    // is no longer part of the note.
                    if matches!(
                        (images, item),
                        (true, Block::Picture(_)) | (false, Block::Table(_))
                    ) && document.shows(item)
                    {
                        sources.push((i, tab, block));
                    }
                }
            }
        }
        sources
    }

    fn open_content_source(&mut self, note: usize, tab: usize) {
        if tab < self.data.notes[note].document.tabs.len() {
            self.data.notes[note].document.active = tab;
            self.data.notes[note].document.tabs[tab].hidden = false;
        }
        self.open_note(note);
    }

    pub fn content_tables(&mut self, ui: &mut egui::Ui) {
        let all = self.content_sources(false);
        if all.is_empty() {
            if empty_with_example(
                ui,
                crate::icons::Icon::Grid,
                "메모에 붙여 넣은 표가 없습니다",
                "표를 복사해 메모 본문에 붙여 넣으면 여기에 모입니다",
            ) {
                self.real_table_gallery(ui);
            }
            return;
        }
        ui.horizontal(|ui| {
            crate::ui::section_header(ui, crate::icons::Icon::Grid, "메모 속 표", Some(all.len()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                crate::ui::search_field(
                    ui,
                    &mut self.extra.table_search,
                    "table_search",
                    "표 안의 내용 검색",
                    300.,
                );
            });
        });
        ui.add_space(8.);
        let query = self.extra.table_search.to_lowercase();
        let mut found = 0;
        for (i, ti, bi) in all {
            let n = &self.data.notes[i];
            let doc = &n.document.tabs[ti];
            let Block::Table(table) = &doc.blocks[bi] else {
                continue;
            };
            if !query.is_empty()
                && !table
                    .cells
                    .iter()
                    .flatten()
                    .any(|c| c.text.to_lowercase().contains(&query))
            {
                continue;
            }
            found += 1;
            let source_title = format!("{} · {}", n.title, doc.title);
            let mut open = false;
            let mut copy = 0;
            ui.push_id(("content_table", i, ti, bi), |ui| {
                crate::ui::card(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(source_title).strong().size(13.5));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if crate::ui::pill(ui, false, "이미지로 복사").clicked() {
                                copy = 2;
                            }
                            if crate::ui::pill(ui, false, "표로 복사").clicked() {
                                copy = 1;
                            }
                            if crate::ui::pill(ui, false, "원본 메모").clicked() {
                                open = true;
                            }
                        });
                    });
                    ui.add_space(8.);
                    egui::ScrollArea::horizontal()
                        .id_salt(("content_table_scroll", i, ti, bi))
                        .show(ui, |ui| {
                            egui::Grid::new(("content_table_cells", i, ti, bi))
                                .striped(true)
                                .min_col_width(110.)
                                .max_col_width(260.)
                                .show(ui, |ui| {
                                    for row in &table.cells {
                                        for cell in row {
                                            let mut text = RichText::new(&cell.text);
                                            if cell.format.bold {
                                                text = text.strong();
                                            }
                                            if cell.format.italic {
                                                text = text.italics();
                                            }
                                            if cell.format.underline {
                                                text = text.underline();
                                            }
                                            if cell.format.strike {
                                                text = text.strikethrough();
                                            }
                                            ui.label(text);
                                        }
                                        ui.end_row();
                                    }
                                });
                        });
                });
            });
            if copy == 1 {
                let result = arboard::Clipboard::new().and_then(|mut clipboard| {
                    clipboard.set_html(table_html(table), Some(table.tsv()))
                });
                self.toast = result
                    .map(|_| "표를 HTML / 탭 구분 텍스트로 복사했습니다".into())
                    .unwrap_or_else(|e| format!("복사 실패: {e}"));
            } else if copy == 2 {
                self.toast = table_image(table)
                    .and_then(|image| {
                        copy_rgba(
                            image.width() as usize,
                            image.height() as usize,
                            image.into_raw(),
                        )
                    })
                    .map(|_| "표 이미지를 복사했습니다".into())
                    .unwrap_or_else(|e| format!("복사 실패: {e}"));
            }
            if open {
                self.open_content_source(i, ti);
            }
            ui.add_space(12.);
        }
        if found == 0 {
            crate::ui::empty_state(
                ui,
                crate::icons::Icon::Search,
                "일치하는 표가 없습니다",
                "다른 검색어로 찾아보세요",
            );
        }
    }

    pub fn content_images(&mut self, ui: &mut egui::Ui) {
        let all = self.content_sources(true);
        if all.is_empty() {
            if empty_with_example(
                ui,
                crate::icons::Icon::Grid,
                "메모에 붙여 넣은 이미지가 없습니다",
                "이미지를 복사해 메모 본문에 붙여 넣으면 여기에 모입니다",
            ) {
                self.real_image_gallery(ui);
            }
            return;
        }
        let bytes: usize = all
            .iter()
            .filter_map(
                |&(i, t, b)| match &self.data.notes[i].document.tabs[t].blocks[b] {
                    Block::Picture(p) => Some(p.pixels.len()),
                    _ => None,
                },
            )
            .sum();
        ui.label(format!(
            "{}개 · 편집 이미지 {:.2} MB · 메모리 기준",
            all.len(),
            bytes as f64 / 1024. / 1024.
        ));
        let mut batch = false;
        ui.horizontal_wrapped(|ui| {
            self.date_filter_button(ui, 6, "시작 날짜", self.extra.media_from);
            self.date_filter_button(ui, 7, "끝 날짜", self.extra.media_to);
            if ui.button("필터 해제").clicked() {
                self.extra.media_from = None;
                self.extra.media_to = None;
            }
            batch = ui.button("일괄 저장하기").clicked();
        });
        let sources: Vec<_> = all
            .into_iter()
            .filter(|&(i, _, _)| {
                let date = self.data.notes[i].created.date();
                self.extra.media_from.is_none_or(|d| date >= d)
                    && self.extra.media_to.is_none_or(|d| date <= d)
            })
            .collect();
        ui.label(
            RichText::new(format!(
                "{}개 표시 · 날짜 기준은 원본 메모 생성일입니다",
                sources.len()
            ))
            .small()
            .color(MUTED),
        );
        ui.add_space(10.);
        if batch {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                let mut saved = 0;
                let mut failed = 0;
                for &(i, t, b) in &sources {
                    let Block::Picture(p) = &self.data.notes[i].document.tabs[t].blocks[b] else {
                        continue;
                    };
                    let stem = format!("memo-{}-{}-{}-{}", i + 1, t + 1, b + 1, safe_stem(&p.name));
                    match picture_png(p).and_then(|bytes| write_unique_png(&folder, &stem, &bytes))
                    {
                        Ok(_) => saved += 1,
                        Err(_) => failed += 1,
                    }
                }
                self.toast = format!(
                    "이미지 {saved}개 저장{}",
                    if failed > 0 {
                        format!(" · {failed}개 실패")
                    } else {
                        String::new()
                    }
                );
            }
        }
        let columns = ((ui.available_width() + 12.) / 245.).floor().clamp(1., 4.) as usize;
        let width =
            ((ui.available_width() - 14. * (columns as f32 - 1.)) / columns as f32).max(180.);
        egui::Grid::new("content_image_gallery")
            .num_columns(columns)
            .spacing([14., 14.])
            .show(ui, |ui| {
                for (at, &(i, t, b)) in sources.iter().enumerate() {
                    let n = &self.data.notes[i];
                    let Block::Picture(picture) = &n.document.tabs[t].blocks[b] else {
                        continue;
                    };
                    let mut copy = false;
                    let mut save = false;
                    let mut open = false;
                    ui.allocate_ui_with_layout(
                        Vec2::new(width, 230.),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            egui::Frame::NONE
                                .fill(ui.visuals().faint_bg_color)
                                .corner_radius(8.)
                                .inner_margin(10.)
                                .show(ui, |ui| {
                                    ui.set_width(width - 20.);
                                    let display_width = width - 20.;
                                    if let Ok(texture) = picture_texture(ui, (i, t, b), picture) {
                                        let factor = (display_width / picture.width as f32)
                                            .min(150. / picture.height as f32);
                                        let size = Vec2::new(
                                            picture.width as f32 * factor,
                                            picture.height as f32 * factor,
                                        );
                                        let response = ui.add(
                                            egui::Image::new(&texture)
                                                .fit_to_exact_size(size)
                                                .sense(egui::Sense::click()),
                                        );
                                        copy = response
                                            .on_hover_text("클릭하면 이미지가 복사됩니다")
                                            .clicked();
                                    } else {
                                        ui.add_sized(
                                            [display_width, 150.],
                                            egui::Label::new("이미지를 표시할 수 없습니다"),
                                        );
                                    }
                                    ui.add(
                                        egui::Label::new(RichText::new(&picture.name).strong())
                                            .truncate(),
                                    );
                                    ui.label(
                                        RichText::new(format!(
                                            "{} · {} × {}",
                                            n.created.format("%Y.%m.%d"),
                                            picture.width,
                                            picture.height
                                        ))
                                        .small()
                                        .color(MUTED),
                                    );
                                    ui.horizontal(|ui| {
                                        save = save_icon(ui).clicked();
                                        open = ui
                                            .small_button("원본 메모")
                                            .on_hover_text(&n.title)
                                            .clicked();
                                    });
                                });
                        },
                    );
                    if copy {
                        self.toast = validate_picture(picture)
                            .and_then(|_| {
                                copy_rgba(picture.width, picture.height, picture.pixels.clone())
                            })
                            .map(|_| "이미지를 복사했습니다".into())
                            .unwrap_or_else(|e| format!("복사 실패: {e}"));
                    }
                    if save {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("PNG 이미지", &["png"])
                            .set_file_name(format!("{}.png", safe_stem(&picture.name)))
                            .save_file()
                        {
                            self.toast = picture_png(picture)
                                .and_then(|bytes| {
                                    std::fs::write(path, bytes).map_err(|e| e.to_string())
                                })
                                .map(|_| "이미지를 저장했습니다".into())
                                .unwrap_or_else(|e| format!("저장 실패: {e}"));
                        }
                    }
                    if open {
                        self.open_content_source(i, t);
                    }
                    if (at + 1) % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        if sources.is_empty() {
            ui.label("선택한 날짜에 생성된 이미지가 없습니다");
        }
    }
}

/// An empty collection: a placeholder plus a toggle that shows example
/// content. Returns whether the example is shown.
fn empty_with_example(
    ui: &mut egui::Ui,
    icon: crate::icons::Icon,
    title: &str,
    hint: &str,
) -> bool {
    let id = ui.id().with(("show_example", title));
    let mut show = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(false));
    crate::ui::card(ui, |ui| {
        ui.set_width(ui.available_width());
        crate::ui::empty_state(ui, icon, title, hint);
        ui.vertical_centered(|ui| {
            if crate::ui::pill(
                ui,
                show,
                if show {
                    "예시 숨기기"
                } else {
                    "예시 보기"
                },
            )
            .clicked()
            {
                show = !show;
            }
            ui.add_space(10.);
        });
    });
    ui.data_mut(|d| d.insert_temp(id, show));
    if show {
        ui.add_space(12.);
    }
    show
}
fn table_html(table: &Table) -> String {
    format!(
        "<table>{}</table>",
        table
            .cells
            .iter()
            .map(|row| format!(
                "<tr>{}</tr>",
                row.iter()
                    .map(|c| format!("<td>{}</td>", html_escape(&c.text)))
                    .collect::<String>()
            ))
            .collect::<String>()
    )
}

fn html_escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn copy_rgba(width: usize, height: usize, pixels: Vec<u8>) -> Result<(), String> {
    arboard::Clipboard::new()
        .and_then(|mut c| {
            c.set_image(arboard::ImageData {
                width,
                height,
                bytes: Cow::Owned(pixels),
            })
        })
        .map_err(|e| e.to_string())
}

fn validate_picture(p: &Picture) -> Result<(), String> {
    if p.width == 0
        || p.height == 0
        || p.width > 8192
        || p.height > 8192
        || p.width.checked_mul(p.height).and_then(|n| n.checked_mul(4)) != Some(p.pixels.len())
    {
        return Err("이미지 크기 또는 픽셀 데이터가 올바르지 않습니다".into());
    }
    Ok(())
}

fn picture_png(p: &Picture) -> Result<Vec<u8>, String> {
    validate_picture(p)?;
    let image = image::RgbaImage::from_raw(p.width as u32, p.height as u32, p.pixels.clone())
        .ok_or("이미지 변환 실패")?;
    let mut bytes = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}

fn picture_texture(
    ui: &egui::Ui,
    source: Source,
    picture: &Picture,
) -> Result<egui::TextureHandle, String> {
    validate_picture(picture)?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    picture.pixels.hash(&mut hash);
    let id = egui::Id::new((
        "content_picture_texture",
        source,
        picture.width,
        picture.height,
        hash.finish(),
    ));
    if let Some(texture) = ui.ctx().data_mut(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return Ok(texture);
    }
    let texture = ui.ctx().load_texture(
        format!("document-image-{}-{}-{}", source.0, source.1, source.2),
        egui::ColorImage::from_rgba_unmultiplied([picture.width, picture.height], &picture.pixels),
        egui::TextureOptions::LINEAR,
    );
    ui.ctx().data_mut(|d| d.insert_temp(id, texture.clone()));
    Ok(texture)
}

fn safe_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("image");
    let safe: String = stem
        .chars()
        .filter(|&c| !c.is_control() && !"<>:\"/\\|?*".contains(c))
        .take(70)
        .collect();
    let safe = safe.trim_matches([' ', '.']);
    if safe.is_empty() {
        "image".into()
    } else {
        safe.into()
    }
}

fn write_unique_png(folder: &Path, stem: &str, bytes: &[u8]) -> Result<PathBuf, String> {
    for suffix in 0..10000 {
        let name = if suffix == 0 {
            format!("{stem}.png")
        } else {
            format!("{stem}-{suffix}.png")
        };
        let path = folder.join(name);
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(bytes).map_err(|e| e.to_string())?;
                return Ok(path);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Err("같은 이름의 파일이 너무 많습니다".into())
}

fn save_icon(ui: &mut egui::Ui) -> egui::Response {
    let (rect, r) = ui.allocate_exact_size(Vec2::new(30., 28.), egui::Sense::click());
    r.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, true, "다른 이름으로 저장")
    });
    let c = rect.center();
    let stroke = egui::Stroke::new(1.5_f32, ui.visuals().text_color());
    for (a, b) in [
        (Vec2::new(0., -7.), Vec2::new(0., 4.)),
        (Vec2::new(-4., 0.), Vec2::new(0., 4.)),
        (Vec2::new(4., 0.), Vec2::new(0., 4.)),
        (Vec2::new(-7., 7.), Vec2::new(7., 7.)),
    ] {
        ui.painter().line_segment([c + a, c + b], stroke);
    }
    r.on_hover_text("다른 이름으로 저장")
}

fn table_image(table: &Table) -> Result<image::RgbaImage, String> {
    static FONT: std::sync::OnceLock<Option<fontdue::Font>> = std::sync::OnceLock::new();
    let font = FONT
        .get_or_init(|| {
            std::fs::read("C:/Windows/Fonts/malgun.ttf")
                .ok()
                .and_then(|b| fontdue::Font::from_bytes(b, fontdue::FontSettings::default()).ok())
        })
        .as_ref()
        .ok_or("표 이미지 복사에 필요한 글꼴을 읽지 못했습니다")?;
    let columns = table.cells.iter().map(Vec::len).max().unwrap_or(0);
    if columns == 0 || table.cells.is_empty() || columns > 20 || table.cells.len() > 100 {
        return Err("복사할 표의 크기를 확인해 주세요".into());
    }
    let width = 180u32;
    let size = 17.;
    let rows: Vec<Vec<Vec<String>>> = table
        .cells
        .iter()
        .map(|row| {
            row.iter()
                .map(|c| wrapped_lines(&c.text, width as f32 - 18., size, font))
                .collect()
        })
        .collect();
    let heights: Vec<_> = rows
        .iter()
        .map(|r| (r.iter().map(Vec::len).max().unwrap_or(1) as u32 * 23 + 16).max(40))
        .collect();
    let total: u32 = heights.iter().sum::<u32>() + 1;
    if total > 8192 {
        return Err("표가 너무 길어 이미지로 복사할 수 없습니다. 표로 복사를 사용해 주세요".into());
    }
    let mut image = image::RgbaImage::from_pixel(
        columns as u32 * width + 1,
        total,
        image::Rgba([255, 254, 252, 255]),
    );
    let mut top = 0;
    for (ri, row) in rows.iter().enumerate() {
        for (ci, cell) in row.iter().enumerate() {
            for (li, line) in cell.iter().enumerate() {
                draw_line(
                    &mut image,
                    font,
                    line,
                    ci as u32 * width + 9,
                    top + 8 + li as u32 * 23,
                    size,
                );
            }
        }
        top += heights[ri];
        for x in 0..image.width() {
            image.put_pixel(x, top, image::Rgba([205, 205, 200, 255]));
        }
    }
    for col in 0..=columns {
        let x = col as u32 * width;
        for y in 0..image.height() {
            image.put_pixel(x, y, image::Rgba([205, 205, 200, 255]));
        }
    }
    for x in 0..image.width() {
        image.put_pixel(x, 0, image::Rgba([205, 205, 200, 255]));
    }
    Ok(image)
}

fn wrapped_lines(text: &str, width: f32, size: f32, font: &fontdue::Font) -> Vec<String> {
    let mut lines = vec![];
    let mut line = String::new();
    let mut advance = 0.;
    for ch in text.chars() {
        if ch == '\n' {
            lines.push(std::mem::take(&mut line));
            advance = 0.;
            continue;
        }
        if ch == '\r' {
            continue;
        }
        let step = font.metrics(ch, size).advance_width;
        if advance + step > width && !line.is_empty() {
            lines.push(std::mem::take(&mut line));
            advance = 0.;
        }
        line.push(ch);
        advance += step;
    }
    lines.push(line);
    lines
}

fn draw_line(
    image: &mut image::RgbaImage,
    font: &fontdue::Font,
    text: &str,
    x: u32,
    y: u32,
    size: f32,
) {
    let mut left = x as i32;
    for ch in text.chars() {
        let (m, raster) = font.rasterize(ch, size);
        for row in 0..m.height {
            for col in 0..m.width {
                let xx = left + m.xmin + col as i32;
                let yy = y as i32 + size as i32 - m.height as i32 - m.ymin + row as i32;
                if xx >= 0 && yy >= 0 && xx < image.width() as i32 && yy < image.height() as i32 {
                    let alpha = raster[row * m.width + col] as f32 / 255.;
                    let p = image.get_pixel_mut(xx as u32, yy as u32);
                    for c in 0..3 {
                        p[c] = (p[c] as f32 * (1. - alpha) + 48. * alpha) as u8;
                    }
                }
            }
        }
        left += m.advance_width.ceil() as i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn table_export_preserves_variable_columns_and_escapes_html() {
        let mut table = Table::new(2, 4);
        table.cells[0][0].text = "<script>&\"".into();
        table.cells[1][3].text = "마지막 열".into();
        let html = table_html(&table);
        assert!(html.contains("&lt;script&gt;&amp;&quot;"));
        assert!(!html.contains("<script>"));
        assert!(table.tsv().contains("마지막 열"));
        let image = table_image(&table).unwrap();
        assert_eq!(image.width(), 721);
        assert!(image.height() >= 81);
    }
    #[test]
    fn collection_excludes_locked_deleted_and_hidden_project_sources() {
        let mut a = App::sample(Local::now().date_naive());
        for n in a.data.notes.iter_mut().take(4) {
            n.document.tabs[0]
                .blocks
                .push(Block::Table(Table::new(1, 1)));
        }
        a.data.notes[0].locked = true;
        a.data.notes[1].deleted = true;
        a.data.notes[2].project = Some(1);
        a.extra.hidden_projects.insert(1);
        assert_eq!(a.content_sources(false), vec![(3, 0, 0)]);
    }
    #[test]
    fn batch_image_export_never_overwrites_an_existing_file() {
        let folder = std::env::temp_dir().join(format!(
            "memonoking-gallery-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&folder).unwrap();
        let first = write_unique_png(&folder, "test", b"first").unwrap();
        let second = write_unique_png(&folder, "test", b"second").unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&first).unwrap(), b"first");
        assert_eq!(std::fs::read(&second).unwrap(), b"second");
        std::fs::remove_file(first).unwrap();
        std::fs::remove_file(second).unwrap();
        std::fs::remove_dir(folder).unwrap();
    }
}
