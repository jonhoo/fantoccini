use std::ops::RangeInclusive;

use webdriver::command::PrintParameters;

use crate::error::PrintConfigurationError;

/// The builder of [`PrintConfiguration`].
#[derive(Debug)]
pub struct PrintConfigurationBuilder {
    orientation: PrintOrientation,
    scale: f64,
    background: bool,
    size: PrintSize,
    margins: PrintMargins,
    page_ranges: Vec<PrintPageRange>,
    shrink_to_fit: bool,
}

impl Default for PrintConfigurationBuilder {
    fn default() -> Self {
        Self {
            orientation: PrintOrientation::default(),
            scale: 1.0,
            background: false,
            size: PrintSize::default(),
            margins: PrintMargins::default(),
            page_ranges: Vec::default(),
            shrink_to_fit: true,
        }
    }
}

impl PrintConfigurationBuilder {
    /// Builds the [`PrintConfiguration`].
    pub fn build(self) -> Result<PrintConfiguration, PrintConfigurationError> {
        let must_be_finite_and_positive = [
            self.scale,
            self.margins.top,
            self.margins.left,
            self.margins.right,
            self.margins.bottom,
            self.size.width,
            self.size.height,
        ];
        if !must_be_finite_and_positive
            .into_iter()
            .all(|n| n.is_finite())
        {
            return Err(PrintConfigurationError::NonFiniteDimensions);
        }
        if !must_be_finite_and_positive
            .into_iter()
            .all(|n| n.is_sign_positive())
        {
            return Err(PrintConfigurationError::NegativeDimensions);
        }

        if self.size.height < PrintSize::MIN.height || self.size.width < PrintSize::MIN.width {
            return Err(PrintConfigurationError::PrintSizeTooSmall);
        }

        if (self.margins.top + self.margins.bottom) >= self.size.height
            || (self.margins.left + self.margins.right) >= self.size.width
        {
            return Err(PrintConfigurationError::DimensionsOverflow);
        }

        Ok(PrintConfiguration {
            orientation: self.orientation,
            scale: self.scale,
            background: self.background,
            size: self.size,
            margins: self.margins,
            page_ranges: self.page_ranges,
            shrink_to_fit: self.shrink_to_fit,
        })
    }

    /// Sets the orientation of the printed page.
    ///
    /// Default: [`PrintOrientation::Portrait`].
    pub fn orientation(mut self, orientation: PrintOrientation) -> Self {
        self.orientation = orientation;

        self
    }

    /// Sets the scale of the printed page.
    ///
    /// Default: 1.
    pub fn scale(mut self, scale: f64) -> Self {
        self.scale = scale;

        self
    }

    /// Sets whether or not to print the backgrounds of the page.
    ///
    /// Default: false.
    pub fn background(mut self, background: bool) -> Self {
        self.background = background;

        self
    }

    /// Sets the size of the printed page.
    ///
    /// Default: [`PrintSize::A4`].
    pub fn size(mut self, size: PrintSize) -> Self {
        self.size = size;

        self
    }

    /// Sets the margins of the printed page.
    ///
    /// Default: `1x1x1x1cm`.
    pub fn margins(mut self, margins: PrintMargins) -> Self {
        self.margins = margins;

        self
    }

    /// Sets ranges of pages to print.
    ///
    /// An empty `ranges` prints all pages, which is the default.
    pub fn page_ranges(mut self, ranges: Vec<PrintPageRange>) -> Self {
        self.page_ranges = ranges;

        self
    }

    /// Sets whether or not to resize the content to fit the page width,
    /// overriding any page width specified in the content of pages to print.
    ///
    /// Default: true.
    pub fn shrink_to_fit(mut self, shrink_to_fit: bool) -> Self {
        self.shrink_to_fit = shrink_to_fit;

        self
    }
}

/// The print configuration.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintConfiguration {
    orientation: PrintOrientation,
    scale: f64,
    background: bool,
    size: PrintSize,
    margins: PrintMargins,
    page_ranges: Vec<PrintPageRange>,
    shrink_to_fit: bool,
}

impl PrintConfiguration {
    /// Creates a [`PrintConfigurationBuilder`] to configure a [`PrintConfiguration`].
    pub fn builder() -> PrintConfigurationBuilder {
        PrintConfigurationBuilder::default()
    }

    pub(crate) fn into_params(self) -> PrintParameters {
        PrintParameters {
            orientation: self.orientation.into_params(),
            scale: self.scale,
            background: self.background,
            page: self.size.into_params(),
            margin: self.margins.into_params(),
            page_ranges: self
                .page_ranges
                .into_iter()
                .map(|page_range| page_range.into_params())
                .collect(),
            shrink_to_fit: self.shrink_to_fit,
        }
    }
}

impl Default for PrintConfiguration {
    fn default() -> Self {
        PrintConfigurationBuilder::default()
            .build()
            .expect("default configuration is buildable")
    }
}

/// The orientation of the print.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PrintOrientation {
    /// Landscape orientation.
    Landscape,
    #[default]
    /// Portrait orientation.
    Portrait,
}

impl PrintOrientation {
    pub(crate) fn into_params(self) -> webdriver::command::PrintOrientation {
        match self {
            Self::Landscape => webdriver::command::PrintOrientation::Landscape,
            Self::Portrait => webdriver::command::PrintOrientation::Portrait,
        }
    }
}

/// The size of the printed page in centimeters.
///
/// Default: [`PrintSize::A4`].
#[derive(Debug, Clone, PartialEq)]
pub struct PrintSize {
    /// The width in centimeters.
    pub width: f64,
    /// The height in centimeters.
    pub height: f64,
}

impl PrintSize {
    /// The standard A4 paper size, which has the dimension of `21.0x29.7cm`.
    pub const A4: Self = Self {
        width: 21.,
        height: 29.7,
    };

    /// The standard US letter paper size, which has the dimension of `21.59x27.94cm`.
    pub const US_LETTER: Self = Self {
        width: 21.59,
        height: 27.94,
    };

    /// The standard US legal paper size, which has the dimension of `21.59x35.56cm`.
    pub const US_LEGAL: Self = Self {
        width: 21.59,
        height: 35.56,
    };

    /// The minimum page size allowed by the Webdriver2 standard, which is `2.54/72cm`.
    pub const MIN: Self = Self {
        // FIXME: use 2.54/72.0 with MSRV >= 1.82
        width: 0.036,
        height: 0.036,
    };

    pub(crate) fn into_params(self) -> webdriver::command::PrintPage {
        webdriver::command::PrintPage {
            width: self.width,
            height: self.height,
        }
    }
}

impl Default for PrintSize {
    fn default() -> Self {
        Self::A4
    }
}

/// The range of the pages to print.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrintPageRange {
    range: RangeInclusive<u64>,
}

impl PrintPageRange {
    /// A single page to print.
    pub const fn single(page: u64) -> Self {
        Self { range: page..=page }
    }

    /// A range of pages to print.
    ///
    /// Returns None if the range start is greater than the range end.
    pub const fn range(range: RangeInclusive<u64>) -> Option<Self> {
        if *range.start() <= *range.end() {
            Some(Self { range })
        } else {
            None
        }
    }

    pub(crate) fn into_params(self) -> webdriver::command::PrintPageRange {
        let (start, end) = self.range.into_inner();

        if start == end {
            webdriver::command::PrintPageRange::Integer(start)
        } else {
            webdriver::command::PrintPageRange::Range(format!("{start}-{end}"))
        }
    }
}

/// The margins of the printed page in centimeters.
///
/// Default: `1x1x1x1cm`.
#[derive(Debug, Clone, PartialEq)]
pub struct PrintMargins {
    /// The top margin in centimeters.
    pub top: f64,
    /// The bottom margin in centimeters.
    pub bottom: f64,
    /// The left margin in centimeters.
    pub left: f64,
    /// The right margin in centimeters.
    pub right: f64,
}

impl PrintMargins {
    pub(crate) fn into_params(self) -> webdriver::command::PrintMargins {
        webdriver::command::PrintMargins {
            top: self.top,
            bottom: self.bottom,
            left: self.left,
            right: self.right,
        }
    }
}

impl Default for PrintMargins {
    fn default() -> Self {
        Self {
            top: 1.0,
            bottom: 1.0,
            left: 1.0,
            right: 1.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::f64::{INFINITY, NAN};

    use crate::{
        error::PrintConfigurationError,
        wd::{PrintConfiguration, PrintMargins, PrintSize},
    };

    #[test]
    fn negative_print_configuration_dimensions() {
        let margins = PrintConfiguration::builder()
            .margins(PrintMargins {
                top: -1.0,
                bottom: 0.0,
                left: 1.0,
                right: 5.4,
            })
            .build();

        let size = PrintConfiguration::builder()
            .size(PrintSize {
                width: -1.0,
                height: 1.0,
            })
            .build();

        assert_eq!(margins, Err(PrintConfigurationError::NegativeDimensions));
        assert_eq!(size, Err(PrintConfigurationError::NegativeDimensions));
    }

    #[test]
    fn non_finite_print_configuration_dimensions() {
        let nan_margins = PrintConfiguration::builder()
            .margins(PrintMargins {
                top: NAN,
                bottom: 0.0,
                left: 1.0,
                right: 5.4,
            })
            .build();

        let nan_size = PrintConfiguration::builder()
            .size(PrintSize {
                width: NAN,
                height: 1.0,
            })
            .build();

        let infinite_margins = PrintConfiguration::builder()
            .margins(PrintMargins {
                top: INFINITY,
                bottom: 0.0,
                left: 1.0,
                right: 5.4,
            })
            .build();

        let infinite_size = PrintConfiguration::builder()
            .size(PrintSize {
                width: INFINITY,
                height: 1.0,
            })
            .build();

        assert_eq!(
            nan_margins,
            Err(PrintConfigurationError::NonFiniteDimensions)
        );
        assert_eq!(nan_size, Err(PrintConfigurationError::NonFiniteDimensions));
        assert_eq!(
            infinite_margins,
            Err(PrintConfigurationError::NonFiniteDimensions)
        );
        assert_eq!(
            infinite_size,
            Err(PrintConfigurationError::NonFiniteDimensions)
        );
    }

    #[test]
    fn overflow_print_configuration_dimensions() {
        let overflow = PrintConfiguration::builder()
            .size(PrintSize {
                width: 10.0,
                height: 5.0,
            })
            .margins(PrintMargins {
                top: 1.0,
                bottom: 1.0,
                left: 5.0,
                right: 5.0,
            })
            .build();

        assert_eq!(overflow, Err(PrintConfigurationError::DimensionsOverflow));
    }

    #[test]
    fn too_small_print_configuration_dimensions() {
        let size_to_small = PrintConfiguration::builder()
            .size(PrintSize {
                width: 0.01,
                height: 5.0,
            })
            .build();

        assert_eq!(
            size_to_small,
            Err(PrintConfigurationError::PrintSizeTooSmall)
        );
    }
}
