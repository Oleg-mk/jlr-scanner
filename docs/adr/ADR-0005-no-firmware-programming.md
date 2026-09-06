# ADR-0005: No ECU firmware programming

- Status: Accepted
- Decision: ECU firmware/VBF programming, bootloader/recovery flashing, immobilizer/key programming, and security programming are forbidden product capabilities.
- Reason: Their risk and authorization model are outside the diagnostic product scope.
- Consequence: These operations must not appear in any transport, protocol, core, or UI implementation.
