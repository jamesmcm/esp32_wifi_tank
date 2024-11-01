use esp_hal::gpio::Output;

enum MotorDriverState {
    Stopped,
    Forward,
    Reverse,
}

pub struct MotorDriver<'a> {
    v_a_pin: Output<'a, esp_hal::gpio::AnyPin>,
    v_b_pin: Output<'a, esp_hal::gpio::AnyPin>,
    g_a_pin: Output<'a, esp_hal::gpio::AnyPin>,
    g_b_pin: Output<'a, esp_hal::gpio::AnyPin>,
    state: MotorDriverState,
}

impl<'a> MotorDriver<'a> {
    pub fn new(
        v_a_pin: Output<'a, esp_hal::gpio::AnyPin>,
        v_b_pin: Output<'a, esp_hal::gpio::AnyPin>,
        g_a_pin: Output<'a, esp_hal::gpio::AnyPin>,
        g_b_pin: Output<'a, esp_hal::gpio::AnyPin>,
    ) -> Self {
        let mut motor_driver = Self {
            v_a_pin,
            v_b_pin,
            g_a_pin,
            g_b_pin,
            state: MotorDriverState::Stopped,
        };
        motor_driver.stop();
        motor_driver
    }

    pub fn forward(&mut self) {
        self.v_a_pin.set_high();
        self.g_a_pin.set_low();
        self.v_b_pin.set_high();
        self.g_b_pin.set_low();
        self.state = MotorDriverState::Forward;
    }

    pub fn backward(&mut self) {
        self.v_a_pin.set_low();
        self.g_a_pin.set_high();
        self.v_b_pin.set_low();
        self.g_b_pin.set_high();
        self.state = MotorDriverState::Reverse;
    }

    pub fn stop(&mut self) {
        self.v_a_pin.set_low();
        self.g_a_pin.set_low();
        self.v_b_pin.set_low();
        self.g_b_pin.set_low();
        self.state = MotorDriverState::Stopped;
    }
}

#[derive(Debug, PartialEq)]
enum MotorsState {
    Stopped,
    Forward,
    Reverse,
    Left,
    Right,
}

pub struct Motors<'a> {
    left_motor: MotorDriver<'a>,
    right_motor: MotorDriver<'a>,
    state: MotorsState,
}

impl<'a> Motors<'a> {
    pub fn new(left_motor: MotorDriver<'a>, right_motor: MotorDriver<'a>) -> Self {
        let mut motors = Self {
            left_motor,
            right_motor,
            state: MotorsState::Stopped,
        };
        motors.stop();
        motors
    }

    pub fn forward(&mut self) {
        if self.state == MotorsState::Forward {
            return;
        }
        self.state = MotorsState::Forward;
        self.left_motor.forward();
        self.right_motor.forward();
    }

    pub fn backward(&mut self) {
        if self.state == MotorsState::Reverse {
            return;
        }
        self.state = MotorsState::Reverse;
        self.left_motor.backward();
        self.right_motor.backward();
    }

    pub fn stop(&mut self) {
        if self.state == MotorsState::Stopped {
            return;
        }
        self.state = MotorsState::Stopped;
        self.left_motor.stop();
        self.right_motor.stop();
    }

    pub fn left(&mut self) {
        if self.state == MotorsState::Left {
            return;
        }
        self.state = MotorsState::Left;
        self.left_motor.backward();
        self.right_motor.forward();
    }

    pub fn right(&mut self) {
        if self.state == MotorsState::Right {
            return;
        }
        self.state = MotorsState::Right;
        self.left_motor.forward();
        self.right_motor.backward();
    }
}
