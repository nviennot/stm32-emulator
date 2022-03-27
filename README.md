STM32 Emulator
==============

This is a work in progress. The end-goal is to simulate 3D printers.

Example usage
--------------

```
$ cd example/
$ cargo run --release -- config.yaml --busy-loop-stop --color=always -v
```

Example output
----------------

```
[main●] example » time cargo run --release -- config.yaml --busy-loop-stop --color=always -v
    Finished release [optimized] target(s) in 0.09s
     Running `/Users/pafy/stm32-emulator/target/release/stm32-emulator config.yaml --busy-loop-stop --color=always -v`
[tsc=00000000 dtsc=+00000000 pc=0x00000000] DEBUG Mapping region start=0x00000000 len=0x1000 name=NULL_guard
[tsc=00000000 dtsc=+00000000 pc=0x00000000] DEBUG Mapping region start=0x08020000 len=0x80000 name=ROM
[tsc=00000000 dtsc=+00000000 pc=0x00000000] INFO  Loading file=saturn-v4.4.3-pj-v5.bin at base=0x08020000
[tsc=00000000 dtsc=+00000000 pc=0x00000000] DEBUG Mapping region start=0x10000000 len=0x18000 name=RAM-CCM
[tsc=00000000 dtsc=+00000000 pc=0x00000000] DEBUG Mapping region start=0x20000000 len=0x20000 name=RAM
[tsc=00000000 dtsc=+00000000 pc=0x00000000] INFO  Starting emulation
[tsc=00321249 dtsc=+00321249 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadJEDECID
[tsc=00321249 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[ef, 40, 16]
[tsc=00321799 dtsc=+00000550 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x120000
[tsc=00321799 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[21, 7b, 1d, ab, fd, db, ce, 1c, d8, 3b, 7b, 90, b4, cb, 59, 4]
[tsc=05077499 dtsc=+04755700 pc=0x08021bd0] DEBUG DMA2 xfer initiated channel=4 peri_addr=0x40011004 peri=USART1 offset=0x0004 reg=DR dir=Read addr=0x20019818 size=267
[tsc=05079883 dtsc=+00002384 pc=0x08021bd0] DEBUG DMA2 xfer initiated channel=4 peri_addr=0x40011004 peri=USART1 offset=0x0004 reg=DR dir=Write addr=0x20019a63 size=18
[tsc=05079883 dtsc=+00000000 pc=0x08021bd0] INFO  usart-probe p=USART1 usart-probe ''
[tsc=05079883 dtsc=+00000000 pc=0x08021bd0] INFO  usart-probe p=USART1 usart-probe 'UART1 init OK'
[tsc=05080697 dtsc=+00000814 pc=0x08021bd0] DEBUG DMA1 xfer initiated channel=4 peri_addr=0x40004804 peri=USART3 offset=0x0004 reg=DR dir=Read addr=0x2001e000 size=1343
[tsc=05096713 dtsc=+00016016 pc=0x08021bd0] DEBUG DMA2 xfer initiated channel=4 peri_addr=0x40011004 peri=USART1 offset=0x0004 reg=DR dir=Write addr=0x20019a63 size=7
[tsc=05096713 dtsc=+00000000 pc=0x08021bd0] INFO  usart-probe p=USART1 usart-probe 'start'
[tsc=05097013 dtsc=+00000300 pc=0x08021bd0] DEBUG DMA2 xfer initiated channel=4 peri_addr=0x40011004 peri=USART1 offset=0x0004 reg=DR dir=Write addr=0x20019a63 size=7
[tsc=05097013 dtsc=+00000000 pc=0x08021bd0] INFO  usart-probe p=USART1 usart-probe 'start'
[tsc=05183468 dtsc=+00086455 pc=0x0805518e] WARN  READ_UNMAPPED addr=0x1fff7a10 size=4
[tsc=05183472 dtsc=+00000004 pc=0x08055196] WARN  READ_UNMAPPED addr=0x1fff7a14 size=4
[tsc=05183476 dtsc=+00000004 pc=0x0805519e] WARN  READ_UNMAPPED addr=0x1fff7a18 size=4
[tsc=05423465 dtsc=+00239989 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x120000
[tsc=05423465 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[21, 7b, 1d, ab, fd, db, ce, 1c, d8, 3b, 7b, 90, b4, cb, 59, 4]
[tsc=05424088 dtsc=+00000623 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x1a178c
[tsc=05424088 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
[tsc=05424440 dtsc=+00000352 pc=0x08029666] DEBUG SPI3 ext-flash rx=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
[tsc=05424698 dtsc=+00000258 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x1a178c
[tsc=05424698 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
[tsc=05425050 dtsc=+00000352 pc=0x08029666] DEBUG SPI3 ext-flash rx=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
[tsc=07952832 dtsc=+02527782 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x1a0000
[tsc=07952832 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]
[tsc=08575246 dtsc=+00622414 pc=0x08029666] INFO  SPI3 ext-flash cmd=ReadData addr=0x02c000
[tsc=08575246 dtsc=+00000000 pc=0x08029666] DEBUG SPI3 ext-flash rx=[8, 20, 30, 6, 42, cc, 5b, f1, 30, 8, 11, 4, 8, 20, 30, 1]
[tsc=08647703 dtsc=+00072457 pc=0x080201f8] INFO  Busy loop reached
[tsc=08647704 dtsc=+00000001 pc=0x080201f8] INFO  Emulation stop
```
