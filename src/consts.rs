#![allow(dead_code, non_camel_case_types)]

// Channel flags.
pub enum ChanFlag {
    CONNECTED           = 0x0001,
    CONNECTING          = 0x0002,
    MARKED_AWAY         = 0x0004,
    END_OF_MOTD         = 0x0008,
    HAS_WHOX            = 0x0010,
    HAS_IDMSG           = 0x0020,
    HIDE_JOIN           = 0x0040,
    HIDE_JOIN_UNSET     = 0x0080,
    BEEP_ON_MESSAGE     = 0x0100,
    BLINK_TRAY          = 0x0200,
    BLINK_TASKBAR       = 0x0400,
    LOGGING             = 0x0800,
    LOGGING_UNSET       = 0x1000,
    SCROLLBACK          = 0x2000,
    SCROLLBACK_UNSET    = 0x4000,
    STRIP_COLORS        = 0x8000,
    STRIP_COLORS_UNSET  =0x10000,    
}

// Channel types.
pub enum ChanType {
    SERVER                   = 1,
    CHANNEL                  = 2,
    DIALOG                   = 3,
    NOTICE                   = 4,
    SNOTICE                  = 5,    
}

// DCC status values.
pub enum DccStatus {
    QUEUED                  = 0,
    ACTIVE                  = 1,
    FAILED                  = 2,
    DONE                    = 3,
    CONNECTING              = 4,
    ABORTED                 = 5,
}

// Table online has these listed with values 0, 1, 1, 1 which can't be
// right. So I made them sequential ints.
pub enum DccType {
    SEND                    = 0,
    RECIEVE                 = 1,
    CHATRECV                = 2,
    CHATSEND                = 3,
}

// The table online has these "flags" listed as sequential ints.
// I need to verify whether the online page is wrong, or my understanding
// of what "flags" means wrt HexChat is wrong.

pub enum IgnFlag {
    PRIVATE              = 0x01,
    NOTICE               = 0x02,
    CHANNEL              = 0x04,
    CTCP                 = 0x08,
    INVITE               = 0x10,
    UNIGNORE             = 0x20,
    NOSAVE               = 0x40,
    DCC                  = 0x80,
}

// IRC color codes. Use these in strings printed to he/xchat.
const IRC_WHITE: &str            = "\x0300";
const IRC_BLACK: &str            = "\x0301";
const IRC_NAVY: &str             = "\x0302";
const IRC_GREEN: &str            = "\x0303";
const IRC_RED: &str              = "\x0304";
const IRC_MAROON: &str           = "\x0305";
const IRC_PURPLE: &str           = "\x0306";
const IRC_OLIVE: &str            = "\x0307";
const IRC_YELLOW: &str           = "\x0308";
const IRC_LIGHT_GREEN: &str      = "\x0309";
const IRC_TEAL: &str             = "\x0310";
const IRC_CYAN: &str             = "\x0311";
const IRC_ROYAL_BLUE: &str       = "\x0312";
const IRC_MAGENTA: &str          = "\x0313";
const IRC_GRAY: &str             = "\x0314";
const IRC_LIGHT_GRAY: &str       = "\x0315";


// IRC text format codes. Use these in strings printed to he/xchat.

const IRC_BOLD: &str               = "\x02"; //"\002";
const IRC_HIDDEN: &str             = "\x08"; //"\010";
const IRC_UNDERLINE: &str          = "\x1F"; //"\037";
const IRC_ORIG_ATTRIBS: &str       = "\x0F"; //"\017";
const IRC_REVERSE_COLOR: &str      = "\x16"; //"\026";
const IRC_BEEP: &str               = "\x07"; //"\007";
const IRC_ITALICS: &str            = "\x1D"; //"\035";


