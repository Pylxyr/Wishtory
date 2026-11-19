/// The `YYYY-MM-DD` part of a pull's time. Times are text (`YYYY-MM-DD HH:MM:SS`); one that is shorter than a
/// date, which only an old import could have left behind, is returned as it is instead of throwing.
String dayOf(String time) => time.length >= 10 ? time.substring(0, 10) : time;

/// The `YYYY-MM` part of a pull's time, or null when the time is too short to have one.
String? monthOf(String time) => time.length >= 7 ? time.substring(0, 7) : null;
