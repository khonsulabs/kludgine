use std::cell::RefCell;
use std::convert::Infallible;

use cosmic_text::{FamilyOwned, Style, Weight};
use figures::units::{Px, UPx};
use figures::{IntoUnsigned, Point, Rect, Size};
use intentional::Cast;
use plotters::coord::Shift;
use plotters::drawing::DrawingArea;
use plotters_backend::{
    BackendColor, BackendCoord, BackendStyle, BackendTextStyle, DrawingErrorKind, FontFamily,
    FontStyle,
};

use crate::shapes::{PathBuilder, Shape, StrokeOptions};
use crate::text::Text;
use crate::{f32_component_to_u8, Color, DrawableExt, Origin, Texture};

impl From<BackendColor> for Color {
    fn from(value: BackendColor) -> Self {
        Self::new(
            value.rgb.0,
            value.rgb.1,
            value.rgb.2,
            f32_component_to_u8(value.alpha.cast()),
        )
    }
}

impl From<Color> for BackendColor {
    fn from(value: Color) -> Self {
        Self {
            alpha: f64::from(value.alpha_f32()),
            rgb: (value.red(), value.green(), value.blue()),
        }
    }
}

fn pt(coord: BackendCoord) -> Point<Px> {
    Point::new(coord.0, coord.1).map(Px::from)
}

fn font_family(family: &FontFamily<'_>) -> FamilyOwned {
    match family {
        FontFamily::Serif => FamilyOwned::Serif,
        FontFamily::SansSerif => FamilyOwned::SansSerif,
        FontFamily::Monospace => FamilyOwned::Monospace,
        FontFamily::Name(name) => FamilyOwned::Name((*name).to_string()),
    }
}

trait BackendStyleExt {
    fn stroke_options(&self) -> StrokeOptions<Px>;
}

impl<T> BackendStyleExt for T
where
    T: BackendStyle,
{
    fn stroke_options(&self) -> StrokeOptions<Px> {
        StrokeOptions::px_wide(self.stroke_width().cast::<i32>()).colored(self.color().into())
    }
}

impl<'render, 'gfx> super::Renderer<'render, 'gfx> {
    /// Returns this renderer as a [`DrawingArea`] compatible with `plotters`.
    #[must_use]
    pub fn as_plot_area(&mut self) -> DrawingArea<PlotterBackend<'_, 'render, 'gfx>, Shift> {
        DrawingArea::from(PlotterBackend(RefCell::new(self)))
    }

    fn apply_text_style<TStyle>(&mut self, style: &TStyle)
    where
        TStyle: BackendTextStyle,
    {
        self.set_font_family(font_family(&style.family()));
        self.set_font_size(Px::from(style.size().cast::<f32>()));
        match style.style() {
            FontStyle::Normal => {
                self.set_font_style(Style::Normal);
                self.set_font_weight(Weight::NORMAL);
            }
            FontStyle::Oblique => {
                self.set_font_style(Style::Oblique);
                self.set_font_weight(Weight::NORMAL);
            }
            FontStyle::Italic => {
                self.set_font_style(Style::Italic);
                self.set_font_weight(Weight::NORMAL);
            }
            FontStyle::Bold => {
                self.set_font_weight(Weight::BOLD);
            }
        }
    }
}

pub struct PlotterBackend<'plot, 'render, 'gfx>(RefCell<&'plot mut super::Renderer<'render, 'gfx>>);

impl plotters::backend::DrawingBackend for PlotterBackend<'_, '_, '_> {
    type ErrorType = Infallible;

    fn get_size(&self) -> (u32, u32) {
        let Size { width, height } = self.0.borrow().size();
        (width.get(), height.get())
    }

    fn ensure_prepared(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn present(&mut self) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        Ok(())
    }

    fn draw_pixel(
        &mut self,
        point: BackendCoord,
        color: BackendColor,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.draw_rect(point, (point.0 + 1, point.1 + 1), &color, true)
    }

    fn draw_line<S: BackendStyle>(
        &mut self,
        from: BackendCoord,
        to: BackendCoord,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        self.0.borrow_mut().draw_shape(
            &PathBuilder::new(pt(from))
                .line_to(pt(to))
                .build()
                .stroke(style.stroke_options()),
        );
        Ok(())
    }

    fn draw_rect<S: BackendStyle>(
        &mut self,
        upper_left: BackendCoord,
        bottom_right: BackendCoord,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let rect = Rect::from_extents(pt(upper_left), pt(bottom_right)).map(Px::from);
        if fill {
            self.0
                .borrow_mut()
                .draw_shape(&Shape::filled_rect(rect, style.color().into()));
        } else {
            self.0
                .borrow_mut()
                .draw_shape(&Shape::stroked_rect(rect, style.stroke_options()));
        }
        Ok(())
    }

    fn draw_path<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        path: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let options = style.stroke_options();
        if options.color.alpha() == 0 {
            return Ok(());
        }

        let mut path = path.into_iter();

        let Some(start) = path.next() else {
            return Ok(());
        };

        let mut builder = PathBuilder::new(pt(start));

        for point in path {
            builder = builder.line_to(pt(point));
        }

        self.0
            .borrow_mut()
            .draw_shape(&builder.build().stroke(options));

        Ok(())
    }

    fn draw_circle<S: BackendStyle>(
        &mut self,
        center: BackendCoord,
        radius: u32,
        style: &S,
        fill: bool,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let radius = Px::new(radius.cast());
        if fill {
            self.0.borrow_mut().draw_shape(&Shape::filled_circle(
                radius,
                style.color().into(),
                Origin::Custom(pt(center)),
            ));
        } else {
            self.0.borrow_mut().draw_shape(&Shape::stroked_circle(
                radius,
                Origin::Custom(pt(center)),
                style.stroke_options(),
            ));
        }
        Ok(())
    }

    fn fill_polygon<S: BackendStyle, I: IntoIterator<Item = BackendCoord>>(
        &mut self,
        vert: I,
        style: &S,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let color = Color::from(style.color());
        if color.alpha() == 0 {
            return Ok(());
        }

        let mut vert = vert.into_iter();

        let Some(start) = vert.next() else {
            return Ok(());
        };

        let mut builder = PathBuilder::new(pt(start));

        for point in vert {
            builder = builder.line_to(pt(point));
        }

        self.0.borrow_mut().draw_shape(&builder.close().fill(color));
        Ok(())
    }

    fn draw_text<TStyle: BackendTextStyle>(
        &mut self,
        text: &str,
        style: &TStyle,
        pos: BackendCoord,
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut gfx = self.0.borrow_mut();
        gfx.apply_text_style(style);

        gfx.draw_text(Text::new(text, style.color().into()).translate_by(pt(pos)));
        Ok(())
    }

    fn estimate_text_size<TStyle: BackendTextStyle>(
        &self,
        text: &str,
        style: &TStyle,
    ) -> Result<(u32, u32), DrawingErrorKind<Self::ErrorType>> {
        let mut gfx = self.0.borrow_mut();
        gfx.apply_text_style(style);
        let size = gfx.measure_text::<Px>(text).size.into_unsigned();
        Ok((size.width.get(), size.height.get()))
    }

    fn blit_bitmap(
        &mut self,
        pos: BackendCoord,
        (iw, ih): (u32, u32),
        rgb: &[u8],
    ) -> Result<(), DrawingErrorKind<Self::ErrorType>> {
        let mut gfx = self.0.borrow_mut();

        let mut rgba = Vec::with_capacity(iw.cast::<usize>() * ih.cast::<usize>() * 4);
        for rgb in rgb.chunks_exact(3) {
            rgba.extend_from_slice(rgb);
            rgba.push(255);
        }

        let texture = Texture::new_with_data(
            &gfx,
            Size::new(iw, ih).map(UPx::from),
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING,
            wgpu::FilterMode::Linear,
            &rgba,
        );
        gfx.draw_texture_at(&texture, pt(pos), 1.0);

        Ok(())
    }
}
