add r0, r1
add r28, r18
ldi r16, 0xf0
ldi r31, 0x41
cp r8, r1
breq equal
equal:
cp r4, r31
breq equal
