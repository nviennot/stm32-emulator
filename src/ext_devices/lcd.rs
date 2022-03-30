// SPDX-License-Identifier: GPL-3.0-or-later

use std::{convert::TryFrom, collections::VecDeque, rc::Rc, cell::RefCell};

use anyhow::Result;
use sdl2::{render::Canvas, video::Window, pixels::PixelFormatEnum, surface::Surface};
use serde::Deserialize;

use crate::{system::System, sdl::Sdl, util::Point};
use super::ExtDevice;

#[derive(Debug, Deserialize)]
pub struct LcdConfig {
    pub peripheral: String,
    pub width: u16,
    pub height: u16,
}

pub struct Lcd {
    pub config: LcdConfig,
    name: String,

    current_position: Point,

    //reply: Option<Reply>,
    cmd: Option<(Command, Vec<u8>)>,
    drawing: bool,
    sdl: Rc<RefCell<Sdl>>,
    canvas: Canvas<Window>,
    framebuffer: Surface<'static>,
}

impl Lcd {
    pub fn new(config: LcdConfig, sdl: &Rc<RefCell<Sdl>>) -> Result<Self> {
        let canvas = sdl.borrow_mut().new_canvas(
            "LCD",
            config.width as u32,
            config.height as u32
        );

        let framebuffer = Surface::new(
            config.width.into(),
            config.height.into(),
            PixelFormatEnum::Index8
        ).unwrap();

        Ok(Self {
            name: "".to_string(),
            config,
            current_position: Point::default(),
            cmd: None,
            drawing: false,
            sdl: sdl.clone(),
            canvas,
            framebuffer,
        })
    }

    #[inline]
    fn get_framebuffer_pixel(&mut self, x: u16, y: u16) -> &mut u8 {
        let x = x as usize;
        let y = y as usize;

        let fb = self.framebuffer.without_lock_mut().unwrap();
        let fb_ptr = fb.as_mut_ptr() as *mut u8;

        unsafe {
            fb_ptr.add(x + y * self.config.width as usize).as_mut().unwrap()
        }
    }


    fn redraw(&mut self, sys: &System) {
        let tc = self.canvas.texture_creator();
        let texture = self.framebuffer.as_texture(&tc).unwrap();
        self.canvas.copy(&texture, None, None).unwrap();

        warn!("Redraw");

        self.canvas.present();
        if !self.sdl.borrow_mut().pump_events() {
            sys.uc.borrow_mut().emu_stop().unwrap();
        }
    }

    fn draw_pixel_pair(&mut self, c: u8) {
        let Point { mut x, mut y } = self.current_position;

        *self.get_framebuffer_pixel(x,y) = c;
        *self.get_framebuffer_pixel(x+1,y) = c;

        x += 2;
        if x >= self.config.width {
            x = x % 2;
            y += 1;

            if y >= self.config.height {
                y = 0;
            }
        }


        self.current_position = Point { x, y };
    }
}


impl ExtDevice<(), u8> for Lcd {
    fn connect_peripheral(&mut self, peri_name: &str) -> String {
        self.name = format!("{} LCD", peri_name);
        self.name.clone()
    }

    fn read(&mut self, _sys: &System, _addr: ()) -> u8 {
        /*
        match self.reply.as_mut() {
            Some(Reply::Data(d)) => {
                d.pop_front().unwrap_or_default()
            }

            Some(Reply::FileContent(addr)) => {
                let c = self.content[*addr];
                *addr = (*addr + 1) % self.config.size;
                c
            }
            None => 0,
        }
        */
        0
    }

    fn write(&mut self, sys: &System, _addr: (), v: u8) {
        if self.drawing {
            self.draw_pixel_pair(v);

            if self.sdl.borrow_mut().should_redraw() || self.current_position == Point::default() {
                self.redraw(sys);
            }

            return;
        }

        if let Some((cmd, mut args)) = self.cmd.take() {
            // We are collecting a command argument
            args.push(v);
            if let Some(_reply) = self.try_process_command(cmd, &args) {
                //self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, args));
            }
        } else if let Some(cmd) = Command::try_from(v).ok() {
            // We are receiving a new command
            if let Some(_reply) = self.try_process_command(cmd, &[]) {
                //self.reply = Some(reply);
            } else {
                self.cmd = Some((cmd, vec![]));
            }
        } else if v != 0xff && v != 0x00 {
            warn!("{} unknown cmd={:02x}", self.name, v);
        }
    }
}

impl Lcd {
    /// Return some reply when the command is processed.
    /// None when command arguments are incomplete.
    fn try_process_command(&mut self, cmd: Command, args: &[u8]) -> Option<Reply> {
        match (cmd, args) {
            (Command::StartDrawing, []) => {
                self.current_position = Point::default();
                self.drawing = true;
                Some(Reply(vec![].into()))
            }
            _ => None,
        }.map(|reply| {
            debug!("{} cmd={:?} args={:02x?} reply={:02x?}",
                self.name, cmd, args, reply);
            reply
        })
    }
}


#[derive(Clone, Copy, Debug, num_enum::TryFromPrimitive)]
#[repr(u8)]
enum Command {
    GetVersion = 0xF0,
    StartDrawing = 0xFB,

    SetPalette = 0xF1,
    GetPalette = 0xF2,
}


#[derive(Debug)]
struct Reply(VecDeque<u8>);
