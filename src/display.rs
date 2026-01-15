use super::*;
pub static LED: LedType = Mutex::new(None);
pub type LedType = Mutex<ThreadModeRawMutex, Option<DisplayPins<'static>>>;

pub struct DisplayPins<'a> {
    pub col: [Output<'a>; 5],
    pub row: [Output<'a>; 5],
}

pub struct InnerDisplayPins {
    pub col: [Peri<'static, AnyPin>; 5],
    pub row: [Peri<'static, AnyPin>; 5],
}

pub type LedArr = [[u8; 5]; 5];
impl<'a> DisplayPins<'a> {
    pub const HEART: LedArr = [
        [0, 5, 0, 5, 0],
        [5, 5, 5, 5, 5],
        [5, 5, 5, 5, 5],
        [0, 5, 5, 5, 0],
        [0, 0, 5, 0, 0],
    ];
    pub const CROSS: LedArr = [
        [5, 0, 0, 0, 5],
        [0, 5, 0, 5, 0],
        [0, 0, 5, 0, 0],
        [0, 5, 0, 5, 0],
        [5, 0, 0, 0, 5],
    ];
    pub const SQUARE: LedArr = [
        [5, 5, 5, 5, 5],
        [5, 0, 0, 0, 5],
        [5, 0, 0, 0, 5],
        [5, 0, 0, 0, 5],
        [5, 5, 5, 5, 5],
    ];
    pub const BRIGHTSQUARE: LedArr = [
        [1, 2, 3, 4, 5],
        [2, 0, 0, 0, 6],
        [3, 0, 0, 0, 7],
        [4, 0, 0, 0, 8],
        [5, 6, 7, 8, 9],
    ];
    pub const DIGITS: [LedArr; 10] = [
        [
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 5],
            [5, 0, 0, 0, 5],
            [5, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
        ],
        [
            [0, 0, 5, 0, 0],
            [5, 5, 5, 0, 0],
            [0, 0, 5, 0, 0],
            [0, 0, 5, 0, 0],
            [5, 5, 5, 5, 5],
        ],
        [
            [5, 5, 5, 5, 0],
            [0, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 0],
            [5, 5, 5, 5, 5],
        ],
        [
            [5, 5, 5, 5, 0],
            [0, 0, 0, 0, 5],
            [5, 5, 5, 5, 0],
            [0, 0, 0, 0, 5],
            [5, 5, 5, 5, 0],
        ],
        [
            [5, 0, 0, 5, 0],
            [5, 0, 0, 5, 0],
            [5, 0, 0, 5, 0],
            [5, 5, 5, 5, 5],
            [0, 0, 0, 5, 0],
        ],
        [
            [5, 5, 5, 5, 5],
            [5, 0, 0, 0, 0],
            [5, 5, 5, 5, 0],
            [0, 0, 0, 0, 5],
            [5, 5, 5, 5, 0],
        ],
        [
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 0],
            [5, 5, 5, 5, 0],
            [5, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
        ],
        [
            [5, 5, 5, 5, 5],
            [0, 0, 0, 0, 5],
            [0, 0, 0, 5, 0],
            [0, 0, 5, 0, 0],
            [0, 0, 5, 0, 0],
        ],
        [
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
        ],
        [
            [0, 5, 5, 5, 0],
            [5, 0, 0, 0, 5],
            [0, 5, 5, 5, 5],
            [0, 0, 0, 0, 5],
            [0, 5, 5, 5, 0],
        ],
    ];
    const OFF_R: Level = Level::Low;
    const OFF_C: Level = Level::High;
    const DRIVE: OutputDrive = OutputDrive::Standard;
    const BRIGHTNESS_LEVELS: u8 = 10;
    const MICRO_DELTA: u64 = 1000;
    const STR_DELAY: u64 =
        (Self::MICRO_DELTA * 2 / (Self::BRIGHTNESS_LEVELS as u64 * Self::BRIGHTNESS_LEVELS as u64));
    pub fn new(col: [Output<'a>; 5], row: [Output<'a>; 5]) -> Self {
        Self { col, row }
    }
    pub async fn blink_one(&mut self, col: usize, row: usize) {
        self.row[row].set_high();
        self.col[col].set_low();
        Timer::after_millis(2).await;
        self.col[col].set_high();
        self.row[row].set_low();
    }
    async fn blink_row(&mut self, col: &[u8; 5], row: usize) {
        self.row[row].set_high();
        for strength in (1..=Self::BRIGHTNESS_LEVELS).rev() {
            for i in col.iter().enumerate() {
                if *i.1 == strength {
                    self.col[i.0].set_low();
                }
            }
            Timer::after_micros(strength as u64 * Self::STR_DELAY).await;
        }
        for i in 0..5 {
            self.col[i].set_high();
        }
        self.row[row].set_low();
    }
    pub async fn display(&mut self, img: &[[u8; 5]; 5]) {
        for (j, i) in img.iter().enumerate() {
            self.blink_row(i, j).await;
        }
    }
}

impl InnerDisplayPins {
    pub fn to_display(self) -> DisplayPins<'static> {
        DisplayPins {
            col: self
                .col
                .map(|x| Output::new(x, DisplayPins::OFF_C, DisplayPins::DRIVE)),
            row: self
                .row
                .map(|x| Output::new(x, DisplayPins::OFF_R, DisplayPins::DRIVE)),
        }
    }
}

//
pub type ImgChannel = Channel<ThreadModeRawMutex, LedState, 64>;
pub type ImgSender = Sender<'static, ThreadModeRawMutex, LedState, 64>;
pub type ImgReceiver = Receiver<'static, ThreadModeRawMutex, LedState, 64>;

//
pub enum LedState {
    NewImg(LedArr),
    Clear,
}
