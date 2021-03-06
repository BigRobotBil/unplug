use super::{Actor, Error, Object, Result};
use crate::common::{NonNoneList, ReadFrom, ReadOptionFrom, WriteOptionTo, WriteTo};
use crate::event::block::BlockId;
use crate::event::script::{Script, ScriptReader, ScriptWriter};
use crate::globals::Libs;
use byteorder::{ByteOrder, ReadBytesExt, WriteBytesExt, BE, LE};
use std::io::{Read, Seek, SeekFrom, Write};
use std::num::NonZeroU32;

const HEADER_SIZE: u32 = 52;

// These should *always* be at the same offsets. Scripts even hardcode references to them.
const EXPECTED_SETTINGS_OFFSET: u32 = HEADER_SIZE;
const EXPECTED_OBJECTS_OFFSET: u32 = EXPECTED_SETTINGS_OFFSET + SETTINGS_SIZE;

#[derive(Debug, Clone, Default)]
struct Header {
    settings_offset: u32,
    objects_offset: u32,
    events_offset: u32,
    on_prologue: Option<NonZeroU32>,
    on_startup: Option<NonZeroU32>,
    on_dead: Option<NonZeroU32>,
    on_pose: Option<NonZeroU32>,
    on_time_cycle: Option<NonZeroU32>,
    on_time_up: Option<NonZeroU32>,
    actors_offset: u32,
    unk_28_offset: Option<NonZeroU32>,
    unk_2c_offset: Option<NonZeroU32>,
    unk_30_offset: Option<NonZeroU32>,
}

impl<R: Read> ReadFrom<R> for Header {
    type Error = Error;
    fn read_from(reader: &mut R) -> Result<Self> {
        let settings_offset = reader.read_u32::<LE>()?;
        let objects_offset = reader.read_u32::<LE>()?;
        if settings_offset != EXPECTED_SETTINGS_OFFSET || objects_offset != EXPECTED_OBJECTS_OFFSET
        {
            return Err(Error::InvalidHeader);
        }
        Ok(Self {
            settings_offset,
            objects_offset,
            events_offset: reader.read_u32::<LE>()?,
            on_prologue: NonZeroU32::new(reader.read_u32::<LE>()?),
            on_startup: NonZeroU32::new(reader.read_u32::<LE>()?),
            on_dead: NonZeroU32::new(reader.read_u32::<LE>()?),
            on_pose: NonZeroU32::new(reader.read_u32::<LE>()?),
            on_time_cycle: NonZeroU32::new(reader.read_u32::<LE>()?),
            on_time_up: NonZeroU32::new(reader.read_u32::<LE>()?),
            actors_offset: reader.read_u32::<LE>()?,
            unk_28_offset: NonZeroU32::new(reader.read_u32::<LE>()?),
            unk_2c_offset: NonZeroU32::new(reader.read_u32::<LE>()?),
            unk_30_offset: NonZeroU32::new(reader.read_u32::<LE>()?),
        })
    }
}

impl<W: Write> WriteTo<W> for Header {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        writer.write_u32::<LE>(self.settings_offset)?;
        writer.write_u32::<LE>(self.objects_offset)?;
        writer.write_u32::<LE>(self.events_offset)?;
        writer.write_u32::<LE>(self.on_prologue.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.on_startup.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.on_dead.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.on_pose.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.on_time_cycle.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.on_time_up.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.actors_offset)?;
        writer.write_u32::<LE>(self.unk_28_offset.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.unk_2c_offset.map(|o| o.get()).unwrap_or(0))?;
        writer.write_u32::<LE>(self.unk_30_offset.map(|o| o.get()).unwrap_or(0))?;
        Ok(())
    }
}

const SETTINGS_SIZE: u32 = 20;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    pub unk_00: i32,
    pub unk_04: u8,
    pub unk_05: u8,
    pub unk_06: i16,
    pub unk_08: u8,
    pub unk_09: u8,
    pub item_flags_base: i16,
    pub coin_flags_base: i16,
    pub dust_flags_base: i16,
    pub unk_10: i16,
    pub unk_12: i16,
}

impl<R: Read> ReadFrom<R> for Settings {
    type Error = Error;
    fn read_from(reader: &mut R) -> Result<Self> {
        Ok(Self {
            unk_00: reader.read_i32::<BE>()?,
            unk_04: reader.read_u8()?,
            unk_05: reader.read_u8()?,
            unk_06: reader.read_i16::<BE>()?,
            unk_08: reader.read_u8()?,
            unk_09: reader.read_u8()?,
            item_flags_base: reader.read_i16::<BE>()?,
            coin_flags_base: reader.read_i16::<BE>()?,
            dust_flags_base: reader.read_i16::<BE>()?,
            unk_10: reader.read_i16::<BE>()?,
            unk_12: reader.read_i16::<BE>()?,
        })
    }
}

impl<W: Write> WriteTo<W> for Settings {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        writer.write_i32::<BE>(self.unk_00)?;
        writer.write_u8(self.unk_04)?;
        writer.write_u8(self.unk_05)?;
        writer.write_i16::<BE>(self.unk_06)?;
        writer.write_u8(self.unk_08)?;
        writer.write_u8(self.unk_09)?;
        writer.write_i16::<BE>(self.item_flags_base)?;
        writer.write_i16::<BE>(self.coin_flags_base)?;
        writer.write_i16::<BE>(self.dust_flags_base)?;
        writer.write_i16::<BE>(self.unk_10)?;
        writer.write_i16::<BE>(self.unk_12)?;
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct EventTable {
    entry_points: Vec<u32>,
}

impl EventTable {
    fn read_from<R: Read>(reader: &mut R, count: usize) -> Result<Self> {
        let mut entry_points = vec![0u32; count];
        reader.read_u32_into::<LE>(&mut entry_points)?;
        Ok(Self { entry_points })
    }
}

impl<W: Write> WriteTo<W> for EventTable {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        let mut bytes = vec![0u8; self.entry_points.len() * 4];
        LE::write_u32_into(&self.entry_points, &mut bytes);
        writer.write_all(&bytes)?;
        writer.write_i32::<LE>(-1)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unk28 {
    unk_00: i32,
    unk_04: i32,
    unk_08: i32,
    unk_0c: i32,
    unk_10: i32,
    unk_14: i32,
    unk_18: i32,
    unk_1c: i32,
    unk_20: i32,
    unk_24: i32,
    unk_28: i32,
    unk_2c: i16,
    unk_2e: i16,
    unk_30: i32,
}

impl<R: Read> ReadOptionFrom<R> for Unk28 {
    type Error = Error;
    fn read_option_from(reader: &mut R) -> Result<Option<Self>> {
        let unk_00 = reader.read_i32::<BE>()?;
        if unk_00 == -1 {
            return Ok(None);
        }
        Ok(Some(Self {
            unk_00,
            unk_04: reader.read_i32::<BE>()?,
            unk_08: reader.read_i32::<BE>()?,
            unk_0c: reader.read_i32::<BE>()?,
            unk_10: reader.read_i32::<BE>()?,
            unk_14: reader.read_i32::<BE>()?,
            unk_18: reader.read_i32::<BE>()?,
            unk_1c: reader.read_i32::<BE>()?,
            unk_20: reader.read_i32::<BE>()?,
            unk_24: reader.read_i32::<BE>()?,
            unk_28: reader.read_i32::<BE>()?,
            unk_2c: reader.read_i16::<BE>()?,
            unk_2e: reader.read_i16::<BE>()?,
            unk_30: reader.read_i32::<BE>()?,
        }))
    }
}

impl<W: Write> WriteTo<W> for Unk28 {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        writer.write_i32::<BE>(self.unk_00)?;
        writer.write_i32::<BE>(self.unk_04)?;
        writer.write_i32::<BE>(self.unk_08)?;
        writer.write_i32::<BE>(self.unk_0c)?;
        writer.write_i32::<BE>(self.unk_10)?;
        writer.write_i32::<BE>(self.unk_14)?;
        writer.write_i32::<BE>(self.unk_18)?;
        writer.write_i32::<BE>(self.unk_1c)?;
        writer.write_i32::<BE>(self.unk_20)?;
        writer.write_i32::<BE>(self.unk_24)?;
        writer.write_i32::<BE>(self.unk_28)?;
        writer.write_i16::<BE>(self.unk_2c)?;
        writer.write_i16::<BE>(self.unk_2e)?;
        writer.write_i32::<BE>(self.unk_30)?;
        Ok(())
    }
}

impl<W: Write> WriteOptionTo<W> for Unk28 {
    type Error = Error;
    fn write_option_to(opt: Option<&Self>, writer: &mut W) -> Result<()> {
        match opt {
            Some(x) => x.write_to(writer),
            None => Ok(writer.write_i32::<BE>(-1)?),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unk2C {
    unk_00: i32,
    unk_04: i32,
    unk_08: i32,
    unk_0c: i32,
    unk_10: i32,
    unk_14: i32,
    unk_18: i32,
    unk_1c: i32,
}

impl<R: Read> ReadOptionFrom<R> for Unk2C {
    type Error = Error;
    fn read_option_from(reader: &mut R) -> Result<Option<Self>> {
        let unk_00 = reader.read_i32::<BE>()?;
        if unk_00 == -1 {
            return Ok(None);
        }
        Ok(Some(Self {
            unk_00,
            unk_04: reader.read_i32::<BE>()?,
            unk_08: reader.read_i32::<BE>()?,
            unk_0c: reader.read_i32::<BE>()?,
            unk_10: reader.read_i32::<BE>()?,
            unk_14: reader.read_i32::<BE>()?,
            unk_18: reader.read_i32::<BE>()?,
            unk_1c: reader.read_i32::<BE>()?,
        }))
    }
}

impl<W: Write> WriteTo<W> for Unk2C {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        writer.write_i32::<BE>(self.unk_00)?;
        writer.write_i32::<BE>(self.unk_04)?;
        writer.write_i32::<BE>(self.unk_08)?;
        writer.write_i32::<BE>(self.unk_0c)?;
        writer.write_i32::<BE>(self.unk_10)?;
        writer.write_i32::<BE>(self.unk_14)?;
        writer.write_i32::<BE>(self.unk_18)?;
        writer.write_i32::<BE>(self.unk_1c)?;
        Ok(())
    }
}

impl<W: Write> WriteOptionTo<W> for Unk2C {
    type Error = Error;
    fn write_option_to(opt: Option<&Self>, writer: &mut W) -> Result<()> {
        match opt {
            Some(x) => x.write_to(writer),
            None => Ok(writer.write_i32::<BE>(-1)?),
        }
    }
}

#[derive(Clone)]
pub struct Stage {
    pub objects: Vec<Object>,
    pub actors: Vec<Actor>,
    pub script: Script,

    /// An event that runs when the stage begins loading.
    pub on_prologue: Option<BlockId>,
    /// An event that runs when the stage is finished loading and about to start.
    pub on_startup: Option<BlockId>,
    /// An event that runs when the player runs out of battery power.
    pub on_dead: Option<BlockId>,
    /// An event that runs when the player presses the pose button.
    pub on_pose: Option<BlockId>,
    /// An event that runs when the time of day cycles between day and night.
    pub on_time_cycle: Option<BlockId>,
    /// This event's meaning is mostly unknown. `ahk.bin` is the only stage to set this event and it
    /// displays a Japanese message which translates to "time up," hence this field's name. It is
    /// unknown whether there is actually a way to trigger this event or if it is a remnant of a
    /// deprecated feature. Despite its name, it does *not* seem related to the in-game timer that
    /// controls the day/night cycle.
    pub on_time_up: Option<BlockId>,

    pub settings: Settings,
    pub unk_28: Vec<Unk28>,
    pub unk_2c: Vec<Unk2C>,
    pub unk_30: Vec<Unk28>,
}

impl Stage {
    pub fn read_from<R: Read + Seek>(reader: &mut R, libs: &Libs) -> Result<Self> {
        let header = Header::read_from(reader)?;

        reader.seek(SeekFrom::Start(header.settings_offset as u64))?;
        let settings = Settings::read_from(reader)?;

        reader.seek(SeekFrom::Start(header.objects_offset as u64))?;
        let mut objects = NonNoneList::<Object>::read_from(reader)?.into_vec();

        reader.seek(SeekFrom::Start(header.events_offset as u64))?;
        let events = EventTable::read_from(reader, objects.len())?.entry_points;

        reader.seek(SeekFrom::Start(header.actors_offset as u64))?;
        let actors = NonNoneList::<Actor>::read_from(reader)?.into_vec();

        let unk_28 = match header.unk_28_offset {
            Some(offset) => {
                reader.seek(SeekFrom::Start(offset.get() as u64))?;
                NonNoneList::<Unk28>::read_from(reader)?.into_vec()
            }
            None => vec![],
        };
        let unk_2c = match header.unk_2c_offset {
            Some(offset) => {
                reader.seek(SeekFrom::Start(offset.get() as u64))?;
                NonNoneList::<Unk2C>::read_from(reader)?.into_vec()
            }
            None => vec![],
        };
        let unk_30 = match header.unk_30_offset {
            Some(offset) => {
                reader.seek(SeekFrom::Start(offset.get() as u64))?;
                NonNoneList::<Unk28>::read_from(reader)?.into_vec()
            }
            None => vec![],
        };

        let mut script = ScriptReader::with_libs(reader, &libs.script, &libs.entry_points);
        let on_prologue = header.on_prologue.map(|o| script.read_event(o.get())).transpose()?;
        let on_startup = header.on_startup.map(|o| script.read_event(o.get())).transpose()?;
        let on_dead = header.on_dead.map(|o| script.read_event(o.get())).transpose()?;
        let on_pose = header.on_pose.map(|o| script.read_event(o.get())).transpose()?;
        let on_time_cycle = header.on_time_cycle.map(|o| script.read_event(o.get())).transpose()?;
        let on_time_up = header.on_time_up.map(|o| script.read_event(o.get())).transpose()?;
        for (obj, &event) in objects.iter_mut().zip(&events) {
            if event != 0 {
                obj.script = Some(script.read_event(event)?);
            }
        }
        Ok(Self {
            objects,
            actors,
            script: script.finish()?,
            on_prologue,
            on_startup,
            on_dead,
            on_pose,
            on_time_cycle,
            on_time_up,
            settings,
            unk_28,
            unk_2c,
            unk_30,
        })
    }
}

impl<W: Write + Seek> WriteTo<W> for Stage {
    type Error = Error;
    fn write_to(&self, writer: &mut W) -> Result<()> {
        assert_eq!(writer.seek(SeekFrom::Current(0))?, 0);

        let mut header = Header::default();
        header.write_to(writer)?;

        header.settings_offset = writer.seek(SeekFrom::Current(0))? as u32;
        assert!(header.settings_offset == EXPECTED_SETTINGS_OFFSET);
        self.settings.write_to(writer)?;

        header.objects_offset = writer.seek(SeekFrom::Current(0))? as u32;
        assert!(header.objects_offset == EXPECTED_OBJECTS_OFFSET);
        NonNoneList((&self.objects).into()).write_to(writer)?;

        header.events_offset = writer.seek(SeekFrom::Current(0))? as u32;
        let mut events = EventTable { entry_points: vec![0; self.objects.len()] };
        events.write_to(writer)?;

        header.actors_offset = writer.seek(SeekFrom::Current(0))? as u32;
        NonNoneList((&self.actors).into()).write_to(writer)?;

        if !self.unk_28.is_empty() {
            header.unk_28_offset = NonZeroU32::new(writer.seek(SeekFrom::Current(0))? as u32);
            NonNoneList((&self.unk_28).into()).write_to(writer)?;
        }
        if !self.unk_2c.is_empty() {
            header.unk_2c_offset = NonZeroU32::new(writer.seek(SeekFrom::Current(0))? as u32);
            NonNoneList((&self.unk_2c).into()).write_to(writer)?;
        }
        if !self.unk_30.is_empty() {
            header.unk_30_offset = NonZeroU32::new(writer.seek(SeekFrom::Current(0))? as u32);
            NonNoneList((&self.unk_30).into()).write_to(writer)?;
        }

        let mut script_writer = ScriptWriter::new(&self.script, writer);
        if let Some(prologue) = self.on_prologue {
            header.on_prologue = NonZeroU32::new(script_writer.write_subroutine(prologue)?);
        }
        if let Some(startup) = self.on_startup {
            header.on_startup = NonZeroU32::new(script_writer.write_subroutine(startup)?);
        }
        if let Some(dead) = self.on_dead {
            header.on_dead = NonZeroU32::new(script_writer.write_subroutine(dead)?);
        }
        if let Some(pose) = self.on_pose {
            header.on_pose = NonZeroU32::new(script_writer.write_subroutine(pose)?);
        }
        if let Some(time_cycle) = self.on_time_cycle {
            header.on_time_cycle = NonZeroU32::new(script_writer.write_subroutine(time_cycle)?);
        }
        if let Some(time_up) = self.on_time_up {
            header.on_time_up = NonZeroU32::new(script_writer.write_subroutine(time_up)?);
        }
        for (i, obj) in self.objects.iter().enumerate() {
            if let Some(event) = obj.script {
                events.entry_points[i] = script_writer.write_subroutine(event)?;
            }
        }
        script_writer.finish()?;

        writer.seek(SeekFrom::Start(0))?;
        header.write_to(writer)?;
        writer.seek(SeekFrom::Start(header.events_offset as u64))?;
        events.write_to(writer)?;
        Ok(())
    }
}
