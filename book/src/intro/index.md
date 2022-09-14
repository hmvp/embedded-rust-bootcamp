# Introduction

Welcome to The Embedded (Rust) Introduction Book: An glance into "Bare Metal" embedded systems, such as Microcontrollers, coincidentally using the Rust
Programming Language.

## Who Embedded Introduction is For

Embedded Introduction is for everyone who wants to know a bit about the intricacies of embedded systems

## Scope

The goals of this book are:

* Get developers an introduction into the intricacies of embedded (Rust) development. i.e. What is GPIO and what are interrupts

This book tries to be as general as possible but to make things easier for both
the readers and the writers it uses the ARM Cortex-M architecture in all its
examples. However, the book doesn't assume that the reader is familiar with this
particular architecture and explains details particular to this architecture
where required.

## Who This Book is For

This book caters towards people with no embedded background and no Rust background.

### Assumptions and Prerequisites

* You are comfortable using a Programming Language.

### Other Resources

If you are unfamiliar with anything mentioned above or if you want more information about a specific topic mentioned in this book you might find some of these resources helpful.

| Topic        | Resource | Description |
|--------------|----------|-------------|
| Rust         | [Rust Book](https://doc.rust-lang.org/book/) | If you are not yet comfortable with Rust, we highly suggest reading this book. |
| Rust, Embedded | [Discovery Book](https://docs.rust-embedded.org/discovery/) | If you have never done any embedded programming, this book might be a better start |
| Rust, Embedded | [Rust Embedded Book](https://docs.rust-embedded.org/book/) | If you are already a bit familiar with Rust or embedded programming, we highly suggest reading this book. |
| Rust, Embedded | [Embedded Rust Bookshelf](https://docs.rust-embedded.org) | Here you can find several other resources provided by Rust's Embedded Working Group. |
| Rust, Embedded | [Embedonomicon](https://docs.rust-embedded.org/embedonomicon/) | The nitty gritty details when doing embedded programming in Rust. |
| Rust, Embedded | [embedded FAQ](https://docs.rust-embedded.org/faq.html) | Frequently asked questions about Rust in an embedded context. |
| Interrupts | [Interrupt](https://en.wikipedia.org/wiki/Interrupt) | - |
| Memory-mapped IO/Peripherals | [Memory-mapped I/O](https://en.wikipedia.org/wiki/Memory-mapped_I/O) | - |
| SPI, UART, RS232, USB, I2C, TTL | [Stack Exchange about SPI, UART, and other interfaces](https://electronics.stackexchange.com/questions/37814/usart-uart-rs232-usb-spi-i2c-ttl-etc-what-are-all-of-these-and-how-do-th) | - |

## How to Use This Book

This book generally assumes that you’re reading it front-to-back. Later
chapters build on concepts in earlier chapters, and earlier chapters may
not dig into details on a topic, revisiting the topic in a later chapter.

After a short introduction the book consists of exercises. These are basically fill in the dots exercises.
Each exercise chapter comes with its own project included in this repository

This book will be using the [Raspberry Pico] development board from
Raspberry.org for the exercises contained within. This board
is based on the ARM Cortex-M architecture, and while basic functionality is
the same across most CPUs based on this architecture, peripherals and other
implementation details of Microcontrollers are different between different
vendors, and often even different between Microcontroller families from the same
vendor.

For this reason, we suggest purchasing the [Raspberry Pico] development board
for the purpose of following the examples in this book.

[Raspberry Pico]: https://www.raspberrypi.com/documentation/microcontrollers/raspberry-pi-pico.html

## Attribution

Some pages from this book are based upon the rust-embedded book found in [this repository] which is developed by the [resources team].
This page is loosely based on this [original page].

[this repository]: https://github.com/rust-embedded/book
[resources team]: https://github.com/rust-embedded/wg#the-resources-team
[original page]: https://docs.rust-embedded.org/book/intro/index.html

## Re-using this material

This book is distributed under the following licenses:

* The code samples and free-standing Cargo projects contained within this book are licensed under the terms of both the [MIT License] and the [Apache License v2.0].
* The written prose, pictures and diagrams contained within this book are licensed under the terms of the Creative Commons [CC-BY-SA v4.0] license.

[MIT License]: https://opensource.org/licenses/MIT
[Apache License v2.0]: http://www.apache.org/licenses/LICENSE-2.0
[CC-BY-SA v4.0]: https://creativecommons.org/licenses/by-sa/4.0/legalcode

TL;DR: If you want to use our text or images in your work, you need to:

* Give the appropriate credit (i.e. mention this book on your slide, and provide a link to the relevant page)
* Provide a link to the [CC-BY-SA v4.0] license
* Indicate if you have changed the material in any way, and make any changes to our material available under the same license

Also, please do let us know if you find this book useful!
