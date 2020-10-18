// test_message.rs

// *************************************************************************
// * Copyright (C) 2020 Daniel Mueller (deso@posteo.net)                   *
// *                                                                       *
// * This program is free software: you can redistribute it and/or modify  *
// * it under the terms of the GNU General Public License as published by  *
// * the Free Software Foundation, either version 3 of the License, or     *
// * (at your option) any later version.                                   *
// *                                                                       *
// * This program is distributed in the hope that it will be useful,       *
// * but WITHOUT ANY WARRANTY; without even the implied warranty of        *
// * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the         *
// * GNU General Public License for more details.                          *
// *                                                                       *
// * You should have received a copy of the GNU General Public License     *
// * along with this program.  If not, see <http://www.gnu.org/licenses/>. *
// *************************************************************************

mod common;

use async_trait::async_trait;

use gui::derive::Widget;
use gui::Handleable;
use gui::Id;
use gui::MutCap;
use gui::Ui;

use crate::common::Event;
use crate::common::Message;
use crate::common::TestWidget;
use crate::common::TestWidgetDataBuilder;


/// Test message sending behavior without a handler present.
#[tokio::test]
async fn no_handler() {
  let (mut ui, root) = Ui::new(
    || TestWidgetDataBuilder::new().build(),
    |id, _cap| Box::new(TestWidget::new(id)),
  );

  let result = ui.send(root, Message::new(1)).await;
  assert_eq!(result, None);
}

fn increment_message(message: Message, _cap: &mut dyn MutCap<Event, Message>) -> Option<Message> {
  Some(Message::new(message.value + 1))
}

/// Try repeatedly sending a message and checking the response.
#[tokio::test]
async fn repeated_send_receive() {
  let (mut ui, root) = Ui::new(
    || TestWidgetDataBuilder::new().build(),
    |id, _cap| Box::new(TestWidget::new(id)),
  );
  let w1 = ui.add_ui_widget(
    root,
    || {
      TestWidgetDataBuilder::new()
        .react_handler(increment_message)
        .build()
    },
    |id, _cap| Box::new(TestWidget::new(id)),
  );

  let result = ui.send(w1, Message::new(42)).await;
  assert_eq!(result, Some(Message::new(43)));

  let result = ui.send(w1, Message::new(4)).await;
  assert_eq!(result, Some(Message::new(5)));
}

// Unfortunately making our TestWidget work with async handlers is
// rather involved. It starts with all the boxing that is required, and
// ends with the fact that we never got lifetimes checking out once a
// &mut MutCap is captured. To that end, we need a dedicated struct
// here.
#[derive(Debug, Widget)]
#[gui(Event = Event, Message = Message)]
pub struct ForwardingWidget {
  id: Id,
  next: Id,
}

impl ForwardingWidget {
  pub fn new(id: Id, next: Id) -> Self {
    Self { id, next }
  }
}

#[async_trait(?Send)]
impl Handleable<Event, Message> for ForwardingWidget {
  async fn react(&self, message: Message, cap: &mut dyn MutCap<Event, Message>) -> Option<Message> {
    cap.send(self.next, Message::new(message.value + 1)).await;
    None
  }
}

static mut FINAL_FORWARDED_VALUE: u64 = 0;

/// Verify that sending a message from a react handler works as
/// expected.
#[tokio::test]
async fn forward_message() {
  let (mut ui, root) = Ui::new(
    || {
      TestWidgetDataBuilder::new()
        .react_handler(|m, _| {
          unsafe {
            FINAL_FORWARDED_VALUE = m.value;
          }
          None
        })
        .build()
    },
    |id, _cap| Box::new(TestWidget::new(id)),
  );
  let w1 = ui.add_ui_widget(
    root,
    || Box::new(()),
    |id, _cap| Box::new(ForwardingWidget::new(id, root)),
  );
  let w2 = ui.add_ui_widget(
    root,
    || Box::new(()),
    |id, _cap| Box::new(ForwardingWidget::new(id, w1)),
  );

  let result = ui.send(w2, Message::new(1337)).await;
  assert_eq!(result, None);
  assert_eq!(unsafe { FINAL_FORWARDED_VALUE }, 1339);
}
