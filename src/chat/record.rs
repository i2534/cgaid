use chrono::NaiveTime;
use core::fmt::Display;
use std::str::FromStr;

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub enum Channel {
    World,
    Region,
    Group,
    Common,
}

impl Display for Channel {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::World => write!(f, "世界"),
            Self::Region => write!(f, "地图"),
            Self::Group => write!(f, "队伍"),
            Self::Common => write!(f, "普通"),
        }
    }
}
impl FromStr for Channel {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "世界" => Ok(Self::World),
            "地图" => Ok(Self::Region),
            "GP" => Ok(Self::Group),
            _ => Ok(Self::Common),
        }
    }
}

#[derive(Debug, Eq, Hash, PartialEq, Clone)]
pub struct Record {
    time: NaiveTime,
    channel: Channel,
    message: String,
}

impl Record {
    const TIME_FORMAT: &'static str = "%H:%M:%S";

    pub fn from(line: &str) -> Option<Self> {
        if line.trim().is_empty() {
            return None;
        }
        let mut parts = line.splitn(2, '丂');
        let time = parts
            .next()
            .and_then(|v| NaiveTime::parse_from_str(v, Record::TIME_FORMAT).ok())?;
        let message = parts.next()?.trim().to_owned();
        let mut channel = Channel::Common;
        if message.starts_with("[") {
            let index = message.find(']').unwrap_or(0);
            if index > 0 {
                channel = message[1..index].parse().ok()?;
            }
        }
        Some(Self {
            time,
            channel,
            message,
        })
    }
    pub fn msg(&self) -> &str {
        &self.message
    }
    pub fn fmt_time(&self) -> String {
        self.time.format(Record::TIME_FORMAT).to_string().to_owned()
    }
    pub fn get_channel(&self) -> &Channel {
        &self.channel
    }
    #[allow(dead_code)]
    pub fn is_channel(&self, channel: Channel) -> bool {
        self.channel == channel
    }
}
impl Display for Record {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(
            f,
            "{}: [{}] {}",
            self.time.format(Record::TIME_FORMAT),
            self.channel,
            self.message
        )
    }
}
impl PartialOrd for Record {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Record {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.time.cmp(&other.time)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_record() {
        let line = "12:34:56丂[世界] 你好";
        let record = Record::from(line).unwrap();
        assert_eq!(record.time, NaiveTime::from_hms(12, 34, 56));
        assert_eq!(record.channel, Channel::World);
        assert_eq!(record.message, "你好");

        let line = "12:34:56丂[地图] 你好";
        let record = Record::from(line).unwrap();
        assert_eq!(record.time, NaiveTime::from_hms(12, 34, 56));
        assert_eq!(record.channel, Channel::Region);
        assert_eq!(record.message, "你好");

        let line = "12:34:56丂[GP] 你好";
        let record = Record::from(line).unwrap();
        assert_eq!(record.time, NaiveTime::from_hms(12, 34, 56));
        assert_eq!(record.channel, Channel::Group);
        assert_eq!(record.message, "你好");

        let line = "12:34:56丂 你好";
        let record = Record::from(line).unwrap();
        assert_eq!(record.time, NaiveTime::from_hms(12, 34, 56));
        assert_eq!(record.channel, Channel::Common);
        assert_eq!(record.message, "你好");
    }

    #[test]
    fn test_record_hash() {
        let line = "12:34:56丂[世界] 你好";
        let record = Record::from(line).unwrap();
        let mut set = std::collections::HashSet::new();
        set.insert(Record::from(line).unwrap());
        assert!(set.contains(&record));
    }

    #[test]
    fn test_record_btree() {
        let lines = vec![
            " 21:40:12丂[世界]盛明兰oO: 半山来个合格车头  大号3带2",
            " 21:40:12丂[世界]盛明兰oO: 半山来个合格车头  大号3带2",
            " 21:40:12丂[世界]盛明兰oO: 半山来个合格车头  大号3带2",
        ];
        let records: std::collections::BTreeSet<_> =
            lines.iter().filter_map(|v| Record::from(v)).collect();
        for r in &records {
            println!("{:?}", r);
        }
        assert_eq!(records.len(), 1);
    }
}
