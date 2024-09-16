#include <cassert>
#include <cstddef>
#include <cstdint>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <ostream>
#include <systemc>

using namespace sc_core;

/*
 * The idea here is to build a very very simple 16 bit CPU...
 */

/* A decoded instruction. */
struct Inst {
  // clang-format off
  enum Opcode { /*    Bits: 15, 14, 13, 12, 11, 10,  9,  8,  7,  6,  5,  4,  3,  2,  1,  0 | Description              */
    ILLEGAL   = 0b00000, /*  0   0   0   0   0   ?   ?   ?   ?   ?   ?   ?   ?   ?   ?   ? | TRAP                     */
    LOAD_IMML = 0b00001, /*  0   0   0   0   1  rd  rd  rd   X   X   X   X   X   X   X   X | rd = zext(x)             */
    LOAD_IMMH = 0b00010, /*  0   0   0   1   0  rd  rd  rd   X   X   X   X   X   X   X   X | rd = (x << 8) | rd       */
    _1        = 0b00011,
    ADD       = 0b00100, /*  0   0   1   0   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra + rb             */
    SUB       = 0b00101, /*  0   0   1   0   1  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra - rb             */
    MUL       = 0b00110, /*  0   0   1   1   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra * rb             */
    _2        = 0b00111,
    AND       = 0b01000, /*  0   1   0   0   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra & rb             */
    OR        = 0b01001, /*  0   1   0   0   1  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra | rb             */
    XOR       = 0b01010, /*  0   1   0   1   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra ^ rb             */
    SET_EQ    = 0b01011, /*  0   1   0   1   1  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra == rb            */
    SET_NE    = 0b01100, /*  0   1   1   0   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra != rb            */
    SET_LT    = 0b01101, /*  0   1   1   0   1  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra  < rb            */
    SET_LE    = 0b01110, /*  0   1   1   1   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = ra <= rb            */
    _3        = 0b01111,
    JUMP      = 0b10000, /*  1   0   0   0   0   ?   ?   ?   x   x   x   x   x   x   x   x | pc = pc + sext(x)          */
    IJUMP     = 0b10001, /*  1   0   0   0   1  ra  ra  ra   ?   ?   ?   ?   ?   ?   ?   ? | pc = rd                    */
    BZERO     = 0b10010, /*  1   0   0   1   0  ra  ra  ra   x   x   x   x   x   x   x   x | if (!ra) pc = pc + sext(x) */
    BNOTZ     = 0b10011, /*  1   0   0   1   1  ra  ra  ra   x   x   x   x   x   x   x   x | if ( ra) pc = pc + sext(x) */
    LOAD      = 0b10100, /*  1   0   1   0   0  rd  rd  rd  ra  ra  ra  rb  rb  rb   ?   ? | rd = memory[ra + rb]       */
    STORE     = 0b10101, /*  1   0   1   0   1   ?   ?   ?  ra  ra  ra  rb  rb  rb   ?   ? | memory[ra] = rb            */
  } Op = ILLEGAL;
  // clang-format on

  Inst(Opcode OpC, int rd, int rs1, int rs2, uint16_t imm)
      : Op(OpC), rd(rd), rs1(rs1), rs2(rs2), imm(imm) {}
  Inst(const Inst &other)
      : Op(other.Op), rd(other.rd), rs1(other.rs1), rs2(other.rs2),
        imm(other.imm) {}
  Inst() : Op(Inst::ILLEGAL), rd(-1), rs1(-1), rs2(-1), imm(0xffff) {}

  Inst &operator=(const Inst &other) {
    Op = other.Op;
    rd = other.rd;
    rs1 = other.rs1;
    rs2 = other.rs2;
    imm = other.imm;
    return *this;
  }

  bool operator==(const Inst &rhs) const {
    return Op == rhs.Op && rd == rhs.rd && rs1 == rhs.rs1 && rs2 == rhs.rs2 &&
           imm == rhs.imm;
  }

  int rd = 0, rs1 = 0, rs2 = 0;
  uint16_t imm = 0;

  uint16_t encode() const {
    switch (Op) {
	case Inst::LOAD_IMML:
	case Inst::LOAD_IMMH:
	  return (uint16_t(Op) << 11) | (uint16_t(rd) << 8) | imm;
	case Inst::ADD:
	case Inst::SUB:
	case Inst::MUL:
	case Inst::AND:
	case Inst::OR:
	case Inst::XOR:
	case Inst::SET_EQ:
	case Inst::SET_NE:
	case Inst::SET_LT:
	case Inst::SET_LE:
	case Inst::LOAD:
	  return (uint16_t(Op) << 11) | (uint16_t(rd) << 8) | (uint16_t(rs1) << 5) | (uint16_t(rs2) << 2);
	case Inst::JUMP:
	  return (uint16_t(Op) << 11) | (imm & 0xff);
	case Inst::IJUMP:
	  return (uint16_t(Op) << 11) | (uint16_t(rs1) << 8);
	case Inst::BZERO:
	case Inst::BNOTZ:
	  return (uint16_t(Op) << 11) | (uint16_t(rs1) << 8) | (imm & 0xff);
	case Inst::STORE:
	  return (uint16_t(Op) << 11) | (uint16_t(rs1) << 5) | (uint16_t(rs2) << 2);
	default:
	  assert("TODO" && false);
	  return 0;
	}
  }
};

std::ostream &operator<<(std::ostream &os, const Inst &I) {
  int rd = I.rd, rs1 = I.rs1, rs2 = I.rs2;
  uint16_t imm = I.imm;
  switch (I.Op) {
  case Inst::LOAD_IMML:
    os << "r" << rd << " = load_imml " << imm;
    break;
  case Inst::LOAD_IMMH:
    os << "r" << rd << " = load_immh " << imm;
    break;
  case Inst::ADD:
    os << "r" << rd << " = add r" << rs1 << ", r" << rs2;
    break;
  case Inst::SUB:
    os << "r" << rd << " = sub r" << rs1 << ", r" << rs2;
    break;
  case Inst::MUL:
    os << "r" << rd << " = mul r" << rs1 << ", r" << rs2;
    break;
  case Inst::AND:
    os << "r" << rd << " = and r" << rs1 << ", r" << rs2;
    break;
  case Inst::OR:
    os << "r" << rd << " =  or r" << rs1 << ", r" << rs2;
    break;
  case Inst::XOR:
    os << "r" << rd << " = xor r" << rs1 << ", r" << rs2;
    break;
  case Inst::SET_EQ:
    os << "r" << rd << " = set_eq r" << rs1 << ", r" << rs2;
    break;
  case Inst::SET_NE:
    os << "r" << rd << " = set_ne r" << rs1 << ", r" << rs2;
    break;
  case Inst::SET_LT:
    os << "r" << rd << " = set_lt r" << rs1 << ", r" << rs2;
    break;
  case Inst::SET_LE:
    os << "r" << rd << " = set_le r" << rs1 << ", r" << rs2;
    break;
  case Inst::JUMP:
    os << "jump (PC + " << int64_t(int8_t(imm)) << ")";
    break;
  case Inst::IJUMP:
    os << "jump r" << rs1;
    break;
  case Inst::BZERO:
    os << "jump (PC + " << uint64_t(int8_t(imm)) << ") if r" << rs1 << " == 0";
    break;
  case Inst::BNOTZ:
    os << "jump (PC + " << uint64_t(int8_t(imm)) << ") if r" << rs1 << " != 0";
    break;
  case Inst::LOAD:
    os << "r" << rd << " = load memory[r" << rs1 << " + r" << rs2 << "]";
    break;
  case Inst::STORE:
    os << "store memory[r" << rs1 << "] = r" << rs2;
    break;
  default:
    os << "ILLEGAL-INSTRUCTION(" << I.Op << ", " << rd << ", " << rs1 << ", " << rs2 << ", imm: " << imm << ")";
  }
  return os;
}

inline void sc_trace(sc_trace_file *&f, const Inst &I, std::string name) {
  sc_trace(f, I.Op, name + ".OpCode");
  sc_trace(f, I.rd, name + ".rd");
  sc_trace(f, I.rs1, name + ".rs1");
  sc_trace(f, I.rs2, name + ".rs2");
  sc_trace(f, I.imm, name + ".imm");
}

struct InstDecoder : public sc_module {
  /* Ports: */
  sc_in<bool> Clk;
  sc_in<bool> Enable;
  sc_in<uint16_t> Raw;
  sc_out<bool> IsLegalInstr;
  sc_out<Inst> DecodedInstr;

  void tick() {
    Inst Decoded(Inst::ILLEGAL, 0, 0, 0, 0);
    if (!Enable.read()) {
      IsLegalInstr.write(0);
      DecodedInstr.write(Decoded);
      return;
    }

    uint16_t RawInst = Raw.read();
    uint16_t imm = uint8_t(RawInst & 0xff);
    int rd = (RawInst & 0b0000011100000000) >> 8;
    int ra = (RawInst & 0b0000000011100000) >> 5;
    int rb = (RawInst & 0b0000000000011100) >> 2;
	Inst::Opcode OpC = Inst::Opcode((RawInst & 0b1111100000000000) >> 11);
    switch (OpC) {
    case Inst::LOAD_IMML:
    case Inst::LOAD_IMMH:
      Decoded = Inst(OpC, rd, -1, -1, imm);
      break;
    case Inst::ADD:
    case Inst::SUB:
    case Inst::MUL:
    case Inst::AND:
    case Inst::OR:
    case Inst::XOR:
    case Inst::SET_EQ:
    case Inst::SET_NE:
    case Inst::SET_LT:
    case Inst::SET_LE:
      Decoded = Inst(OpC, rd, ra, rb, 0xffff);
      break;
    case Inst::JUMP:
      Decoded = Inst(Inst::JUMP, -1, -1, -1, imm);
      break;
    case Inst::IJUMP:
      Decoded = Inst(Inst::IJUMP, -1, rd, -1, 0xffff);
      break;
    case Inst::BZERO:
    case Inst::BNOTZ:
      Decoded = Inst(OpC, -1, ra, -1, imm);
      break;
    case Inst::LOAD:
      Decoded = Inst(Inst::LOAD, rd, ra, rb, 0xffff);
      break;
    case Inst::STORE:
      Decoded = Inst(Inst::STORE, -1, ra, rb, 0xffff);
      break;
    default:
      DecodedInstr.write(Inst(Inst::ILLEGAL, -1, -1, -1, 0xffff));
      IsLegalInstr.write(0);
      return;
    }

    DecodedInstr.write(Decoded);
    IsLegalInstr.write(1);
  }

  SC_CTOR(InstDecoder)
      : Clk("clock"), Enable("enable"), Raw("raw-inst"),
        IsLegalInstr("is-legal-instr"), DecodedInstr("decoded") {
    SC_METHOD(tick);
    sensitive << Clk;
  }
};

struct RAM: public sc_module {
  /* Ports: */
  sc_in<bool> Clk;
  sc_in<bool> Enable;
  sc_in<bool> DoRead;
  sc_in<bool> DoWrite;
  sc_in<uint16_t> Address;
  sc_inout<uint16_t> DataPort;

  /* Internals: */
  uint16_t Data[(1 << 16)];

  void tick() {
    assert(!DoRead.read() || !DoWrite.read());

    if (!Enable.read())
      return;

    if (DoRead.read()) {
      DataPort.write(Data[Address.read()]);
    }

    if (DoWrite.read()) {
      Data[Address.read()] = DataPort.read();
    }
  }

  SC_CTOR(RAM) {
    SC_METHOD(tick);
    sensitive << Clk;
    memset(&Data[0], 0, (1 << 16) * sizeof(uint16_t));
  }
};

struct CPU: public sc_module {
  /* Ports: */
  sc_in<bool> Clk;
  sc_inout<uint16_t> PC;
  sc_in<bool> Enable;
  sc_in<bool> Reset;

  /* Signals: */
  sc_signal<bool> MemoryDoRead;
  sc_signal<bool> MemoryDoWrite;
  sc_signal<uint16_t> MemoryAddress;
  sc_signal<uint16_t> MemoryData;
  sc_signal<bool> IsLegalInstr;
  sc_signal<uint16_t> RawInstruction;
  sc_signal<Inst> DecodedInstr;

  /* Submodules: */
  RAM Memory;
  InstDecoder Decoder;

  /* Internals: */
  uint16_t registers[(1 << 3)];

  SC_CTOR(CPU)
      : Clk("clock"), PC("PC"), Enable("enable"), Memory("RAM"),
        Decoder("inst-decoder") {

    sensitive << Clk;
    sensitive << Reset;
    SC_THREAD(tick);

    Memory.Clk(Clk);
    Memory.Enable(Enable);
    Memory.DoRead(MemoryDoRead);
    Memory.DoWrite(MemoryDoWrite);
    Memory.Address(MemoryAddress);
    Memory.DataPort(MemoryData);

    Decoder.Clk(Clk);
    Decoder.Enable(Enable);
    Decoder.Raw(RawInstruction);
    Decoder.DecodedInstr(DecodedInstr);
    Decoder.IsLegalInstr(IsLegalInstr);
  }

  // TODO: Pipeline!
  void tick() {
reset:
    memset(&registers[0], 0, sizeof(registers));
    PC = 0x0;
	wait(1, SC_NS);

    for (;;) {
      if (Reset)
        goto reset;
      if (!Enable) {
		wait(1, SC_NS);
        continue;
	  }

      MemoryDoRead = true;
      MemoryDoWrite = false;
      MemoryAddress = PC.read();
      wait(1, SC_NS);
      RawInstruction = MemoryData.read();
      wait(1, SC_NS);
      Inst I = DecodedInstr.read();
      std::cerr << "PC[\t" << PC.read() << "]: " << I << "\n";
	  uint16_t NextPC = PC.read() + 1;
	  bool DoWait = true;
      switch (I.Op) {
      case Inst::LOAD_IMML:
        registers[I.rd] = I.imm;
        break;
      case Inst::LOAD_IMMH:
		registers[I.rd] |= (I.imm << 8);
		break;
	  case Inst::ADD:
		registers[I.rd] = registers[I.rs1] + registers[I.rs2];
		break;
	  case Inst::SUB:
		registers[I.rd] = registers[I.rs1] - registers[I.rs2];
		break;
	  case Inst::MUL:
		registers[I.rd] = registers[I.rs1] * registers[I.rs2];
		break;
	  case Inst::AND:
		registers[I.rd] = registers[I.rs1] & registers[I.rs2];
		break;
	  case Inst::OR:
		registers[I.rd] = registers[I.rs1] | registers[I.rs2];
		break;
	  case Inst::XOR:
		registers[I.rd] = registers[I.rs1] ^ registers[I.rs2];
		break;
	  case Inst::SET_EQ:
		registers[I.rd] = registers[I.rs1] == registers[I.rs2];
		break;
	  case Inst::SET_NE:
		registers[I.rd] = registers[I.rs1] != registers[I.rs2];
		break;
	  case Inst::SET_LT:
		registers[I.rd] = registers[I.rs1]  < registers[I.rs2];
		break;
	  case Inst::SET_LE:
		registers[I.rd] = registers[I.rs1] <= registers[I.rs2];
		break;
	  case Inst::JUMP:
		NextPC = (int16_t(PC) + int8_t(I.imm));
		break;
	  case Inst::IJUMP:
		NextPC = registers[I.rs1];
		break;
	  case Inst::BZERO:
		if (registers[I.rs1] == 0)
			NextPC = ((int16_t(PC) + int8_t(I.imm)));
		break;
	  case Inst::BNOTZ:
		if (registers[I.rs1] != 0)
			NextPC = ((int16_t(PC) + int8_t(I.imm)));
		break;
	  case Inst::LOAD:
		MemoryDoWrite = false;
		MemoryDoRead = true;
		MemoryAddress = registers[I.rs1] + registers[I.rs2];
		DoWait = false;
		wait(1, SC_NS);
		registers[I.rd] = MemoryData.read();
		break;
	  case Inst::STORE:
		MemoryDoWrite = true;
		MemoryDoRead = false;
		MemoryAddress = registers[I.rs1];
		MemoryData.write(registers[I.rs2]);
		DoWait = false;
		wait(1, SC_NS);
		break;
	  default:
		assert("illegal instruction" && false);
		break;
	  }
	  PC.write(NextPC);
	  if (DoWait)
		wait(1, SC_NS);
    }
  }
};

static void initialize_program(uint16_t PC, uint16_t *Mem) {
	Mem[PC + 0] = Inst(Inst::LOAD_IMML, 0x0, -1, -1, 0).encode();
	Mem[PC + 1] = Inst(Inst::LOAD_IMML, 0x1, -1, -1, 1).encode();
	Mem[PC + 2] = Inst(Inst::LOAD_IMML, 0x3, -1, -1, 0).encode();
	Mem[PC + 3] = Inst(Inst::ADD, 0x3, 0x3, 0x1, 0).encode();
	Mem[PC + 4] = Inst(Inst::JUMP, -1, -1, -1, int8_t(-1)).encode();
}

int sc_main(int argc, char **argv) {
  (void)argc;
  (void)argv;

  sc_clock Clock("clock", 1, SC_NS);
  sc_signal<bool> Reset("reset", 1);
  sc_signal<bool> Enable("enable", 0);
  sc_signal<uint16_t> PC("pc", 0);
  CPU cpu("cpu");
  cpu.Clk(Clock);
  cpu.Reset(Reset);
  cpu.Enable(Enable);
  cpu.PC(PC);
  initialize_program(0, cpu.Memory.Data);
  sc_start(10, SC_NS);
  Enable = true;
  Reset = false;

  sc_start(1000, SC_NS);
  printf("Simulation successful!\n");
  for (size_t reg = 0; reg < 8; reg++)
    printf("reg[%ld] = %#04x\n", reg, cpu.registers[reg]);

  return EXIT_SUCCESS;
}
