mod opcode {
    pub const NOP: u8 = 0x00;
    pub const MOVE: u8 = 0x01;
    pub const MOVE_FROM16: u8 = 0x02;
    pub const MOVE_16: u8 = 0x03;
    pub const MOVE_WIDE: u8 = 0x04;
    pub const MOVE_WIDE_FROM16: u8 = 0x05;
    pub const MOVE_WIDE_16: u8 = 0x06;
    pub const MOVE_OBJECT: u8 = 0x07;
    pub const MOVE_OBJECT_FROM16: u8 = 0x08;
    pub const MOVE_OBJECT_16: u8 = 0x09;
    pub const MOVE_RESULT: u8 = 0x0a;
    pub const MOVE_RESULT_WIDE: u8 = 0x0b;
    pub const MOVE_RESULT_OBJECT: u8 = 0x0c;
    pub const MOVE_EXCEPTION: u8 = 0x0d;
    pub const RETURN_VOID: u8 = 0x0e;
    pub const RETURN: u8 = 0x0f;
    pub const RETURN_WIDE: u8 = 0x10;
    pub const RETURN_OBJECT: u8 = 0x11;
    pub const CONST_4: u8 = 0x12;
    pub const CONST_16: u8 = 0x13;
    pub const CONST: u8 = 0x14;
    pub const CONST_HIGH16: u8 = 0x15;
    pub const CONST_WIDE_16: u8 = 0x16;
    pub const CONST_WIDE_32: u8 = 0x17;
    pub const CONST_WIDE: u8 = 0x18;
    pub const CONST_WIDE_HIGH16: u8 = 0x19;
    pub const CONST_STRING: u8 = 0x1a;
    pub const CONST_STRING_JUMBO: u8 = 0x1b;
    pub const CONST_CLASS: u8 = 0x1c;
    pub const MONITOR_ENTER: u8 = 0x1d;
    pub const MONITOR_EXIT: u8 = 0x1e;
    pub const CHECK_CAST: u8 = 0x1f;
    pub const INSTANCE_OF: u8 = 0x20;
    pub const ARRAY_LENGTH: u8 = 0x21;
    pub const NEW_INSTANCE: u8 = 0x22;
    pub const NEW_ARRAY: u8 = 0x23;
    pub const FILLED_NEW_ARRAY: u8 = 0x24;
    pub const FILLED_NEW_ARRAY_RANGE: u8 = 0x25;
    pub const FILL_ARRAY_DATA: u8 = 0x26;
    pub const THROW: u8 = 0x27;
    pub const GOTO: u8 = 0x28;
    pub const GOTO_16: u8 = 0x29;
    pub const GOTO_32: u8 = 0x2a;
    pub const PACKED_SWITCH: u8 = 0x2b;
    pub const SPARSE_SWITCH: u8 = 0x2c;
    pub const CMPL_FLOAT: u8 = 0x2d;
    pub const CMPG_FLOAT: u8 = 0x2e;
    pub const CMPL_DOUBLE: u8 = 0x2f;
    pub const CMPG_DOUBLE: u8 = 0x30;
    pub const CMP_LONG: u8 = 0x31;
    pub const IF_EQ: u8 = 0x32;
    pub const IF_NE: u8 = 0x33;
    pub const IF_LT: u8 = 0x34;
    pub const IF_GE: u8 = 0x35;
    pub const IF_GT: u8 = 0x36;
    pub const IF_LE: u8 = 0x37;
    pub const IF_EQZ: u8 = 0x38;
    pub const IF_NEZ: u8 = 0x39;
    pub const IF_LTZ: u8 = 0x3a;
    pub const IF_GEZ: u8 = 0x3b;
    pub const IF_GTZ: u8 = 0x3c;
    pub const IF_LEZ: u8 = 0x3d;
    pub const AGET: u8 = 0x44;
    pub const AGET_WIDE: u8 = 0x45;
    pub const AGET_OBJECT: u8 = 0x46;
    pub const AGET_BOOLEAN: u8 = 0x47;
    pub const AGET_BYTE: u8 = 0x48;
    pub const AGET_CHAR: u8 = 0x49;
    pub const AGET_SHORT: u8 = 0x4a;
    pub const APUT: u8 = 0x4b;
    pub const APUT_WIDE: u8 = 0x4c;
    pub const APUT_OBJECT: u8 = 0x4d;
    pub const APUT_BOOLEAN: u8 = 0x4e;
    pub const APUT_BYTE: u8 = 0x4f;
    pub const APUT_CHAR: u8 = 0x50;
    pub const APUT_SHORT: u8 = 0x51;
    pub const IGET: u8 = 0x52;
    pub const IGET_WIDE: u8 = 0x53;
    pub const IGET_OBJECT: u8 = 0x54;
    pub const IGET_BOOLEAN: u8 = 0x55;
    pub const IGET_BYTE: u8 = 0x56;
    pub const IGET_CHAR: u8 = 0x57;
    pub const IGET_SHORT: u8 = 0x58;
    pub const IPUT: u8 = 0x59;
    pub const IPUT_WIDE: u8 = 0x5a;
    pub const IPUT_OBJECT: u8 = 0x5b;
    pub const IPUT_BOOLEAN: u8 = 0x5c;
    pub const IPUT_BYTE: u8 = 0x5d;
    pub const IPUT_CHAR: u8 = 0x5e;
    pub const IPUT_SHORT: u8 = 0x5f;
    pub const SGET: u8 = 0x60;
    pub const SGET_WIDE: u8 = 0x61;
    pub const SGET_OBJECT: u8 = 0x62;
    pub const SGET_BOOLEAN: u8 = 0x63;
    pub const SGET_BYTE: u8 = 0x64;
    pub const SGET_CHAR: u8 = 0x65;
    pub const SGET_SHORT: u8 = 0x66;
    pub const SPUT: u8 = 0x67;
    pub const SPUT_WIDE: u8 = 0x68;
    pub const SPUT_OBJECT: u8 = 0x69;
    pub const SPUT_BOOLEAN: u8 = 0x6a;
    pub const SPUT_BYTE: u8 = 0x6b;
    pub const SPUT_CHAR: u8 = 0x6c;
    pub const SPUT_SHORT: u8 = 0x6d;
    pub const INVOKE_VIRTUAL: u8 = 0x6e;
    pub const INVOKE_SUPER: u8 = 0x6f;
    pub const INVOKE_DIRECT: u8 = 0x70;
    pub const INVOKE_STATIC: u8 = 0x71;
    pub const INVOKE_INTERFACE: u8 = 0x72;
    pub const INVOKE_VIRTUAL_RANGE: u8 = 0x74;
    pub const INVOKE_SUPER_RANGE: u8 = 0x75;
    pub const INVOKE_DIRECT_RANGE: u8 = 0x76;
    pub const INVOKE_STATIC_RANGE: u8 = 0x77;
    pub const INVOKE_INTERFACE_RANGE: u8 = 0x78;
    pub const INVOKE_POLYMORPHIC: u8 = 0xfa;
    pub const INVOKE_POLYMORPHIC_RANGE: u8 = 0xfb;
    pub const INVOKE_CUSTOM: u8 = 0xfc;
    pub const INVOKE_CUSTOM_RANGE: u8 = 0xfd;
    pub const CONST_METHOD_HANDLE: u8 = 0xfe;
    pub const CONST_METHOD_TYPE: u8 = 0xff;
    pub const NEG_INT: u8 = 0x7b;
    pub const NOT_INT: u8 = 0x7c;
    pub const NEG_LONG: u8 = 0x7d;
    pub const NOT_LONG: u8 = 0x7e;
    pub const NEG_FLOAT: u8 = 0x7f;
    pub const NEG_DOUBLE: u8 = 0x80;
    pub const INT_TO_LONG: u8 = 0x81;
    pub const INT_TO_FLOAT: u8 = 0x82;
    pub const INT_TO_DOUBLE: u8 = 0x83;
    pub const LONG_TO_INT: u8 = 0x84;
    pub const LONG_TO_FLOAT: u8 = 0x85;
    pub const LONG_TO_DOUBLE: u8 = 0x86;
    pub const FLOAT_TO_INT: u8 = 0x87;
    pub const FLOAT_TO_LONG: u8 = 0x88;
    pub const FLOAT_TO_DOUBLE: u8 = 0x89;
    pub const DOUBLE_TO_INT: u8 = 0x8a;
    pub const DOUBLE_TO_LONG: u8 = 0x8b;
    pub const DOUBLE_TO_FLOAT: u8 = 0x8c;
    pub const INT_TO_BYTE: u8 = 0x8d;
    pub const INT_TO_CHAR: u8 = 0x8e;
    pub const INT_TO_SHORT: u8 = 0x8f;
    pub const ADD_INT: u8 = 0x90;
    pub const SUB_INT: u8 = 0x91;
    pub const MUL_INT: u8 = 0x92;
    pub const DIV_INT: u8 = 0x93;
    pub const REM_INT: u8 = 0x94;
    pub const AND_INT: u8 = 0x95;
    pub const OR_INT: u8 = 0x96;
    pub const XOR_INT: u8 = 0x97;
    pub const SHL_INT: u8 = 0x98;
    pub const SHR_INT: u8 = 0x99;
    pub const USHR_INT: u8 = 0x9a;
    pub const ADD_LONG: u8 = 0x9b;
    pub const SUB_LONG: u8 = 0x9c;
    pub const MUL_LONG: u8 = 0x9d;
    pub const DIV_LONG: u8 = 0x9e;
    pub const REM_LONG: u8 = 0x9f;
    pub const AND_LONG: u8 = 0xa0;
    pub const OR_LONG: u8 = 0xa1;
    pub const XOR_LONG: u8 = 0xa2;
    pub const SHL_LONG: u8 = 0xa3;
    pub const SHR_LONG: u8 = 0xa4;
    pub const USHR_LONG: u8 = 0xa5;
    pub const ADD_FLOAT: u8 = 0xa6;
    pub const SUB_FLOAT: u8 = 0xa7;
    pub const MUL_FLOAT: u8 = 0xa8;
    pub const DIV_FLOAT: u8 = 0xa9;
    pub const REM_FLOAT: u8 = 0xaa;
    pub const ADD_DOUBLE: u8 = 0xab;
    pub const SUB_DOUBLE: u8 = 0xac;
    pub const MUL_DOUBLE: u8 = 0xad;
    pub const DIV_DOUBLE: u8 = 0xae;
    pub const REM_DOUBLE: u8 = 0xaf;
    pub const ADD_INT_2ADDR: u8 = 0xb0;
    pub const SUB_INT_2ADDR: u8 = 0xb1;
    pub const MUL_INT_2ADDR: u8 = 0xb2;
    pub const DIV_INT_2ADDR: u8 = 0xb3;
    pub const REM_INT_2ADDR: u8 = 0xb4;
    pub const AND_INT_2ADDR: u8 = 0xb5;
    pub const OR_INT_2ADDR: u8 = 0xb6;
    pub const XOR_INT_2ADDR: u8 = 0xb7;
    pub const SHL_INT_2ADDR: u8 = 0xb8;
    pub const SHR_INT_2ADDR: u8 = 0xb9;
    pub const USHR_INT_2ADDR: u8 = 0xba;
    pub const ADD_LONG_2ADDR: u8 = 0xbb;
    pub const SUB_LONG_2ADDR: u8 = 0xbc;
    pub const MUL_LONG_2ADDR: u8 = 0xbd;
    pub const DIV_LONG_2ADDR: u8 = 0xbe;
    pub const REM_LONG_2ADDR: u8 = 0xbf;
    pub const AND_LONG_2ADDR: u8 = 0xc0;
    pub const OR_LONG_2ADDR: u8 = 0xc1;
    pub const XOR_LONG_2ADDR: u8 = 0xc2;
    pub const SHL_LONG_2ADDR: u8 = 0xc3;
    pub const SHR_LONG_2ADDR: u8 = 0xc4;
    pub const USHR_LONG_2ADDR: u8 = 0xc5;
    pub const ADD_FLOAT_2ADDR: u8 = 0xc6;
    pub const SUB_FLOAT_2ADDR: u8 = 0xc7;
    pub const MUL_FLOAT_2ADDR: u8 = 0xc8;
    pub const DIV_FLOAT_2ADDR: u8 = 0xc9;
    pub const REM_FLOAT_2ADDR: u8 = 0xca;
    pub const ADD_DOUBLE_2ADDR: u8 = 0xcb;
    pub const SUB_DOUBLE_2ADDR: u8 = 0xcc;
    pub const MUL_DOUBLE_2ADDR: u8 = 0xcd;
    pub const DIV_DOUBLE_2ADDR: u8 = 0xce;
    pub const REM_DOUBLE_2ADDR: u8 = 0xcf;
    pub const ADD_INT_LIT16: u8 = 0xd0;
    pub const RSUB_INT: u8 = 0xd1;
    pub const MUL_INT_LIT16: u8 = 0xd2;
    pub const DIV_INT_LIT16: u8 = 0xd3;
    pub const REM_INT_LIT16: u8 = 0xd4;
    pub const AND_INT_LIT16: u8 = 0xd5;
    pub const OR_INT_LIT16: u8 = 0xd6;
    pub const XOR_INT_LIT16: u8 = 0xd7;
    pub const ADD_INT_LIT8: u8 = 0xd8;
    pub const RSUB_INT_LIT8: u8 = 0xd9;
    pub const MUL_INT_LIT8: u8 = 0xda;
    pub const DIV_INT_LIT8: u8 = 0xdb;
    pub const REM_INT_LIT8: u8 = 0xdc;
    pub const AND_INT_LIT8: u8 = 0xdd;
    pub const OR_INT_LIT8: u8 = 0xde;
    pub const XOR_INT_LIT8: u8 = 0xdf;
    pub const SHL_INT_LIT8: u8 = 0xe0;
    pub const SHR_INT_LIT8: u8 = 0xe1;
    pub const USHR_INT_LIT8: u8 = 0xe2;
}

use crate::input::operands::RawLiteral;
use crate::ir::insn::FillArrayData;
use crate::ir::{
    ArgType, ArithOp, CmpBias, IfOp, InsnArg, InsnNode, InvokeType, RegNum, RegisterArg, UnaryOp,
};
use crate::p1::{DecodedInstruction, DecodedMethod};
use crate::{CodeUnitOffset, RegisterId};

/// Translate one checked P1 instruction into the recovery IR.
///
/// Operands, widths, branch targets, payload targets and reference indices all
/// come from the published P1 item. No original word is decoded again.
pub(super) fn decode(facts: &DecodedMethod<'_>, item: &DecodedInstruction) -> Option<InsnNode> {
    use opcode::*;

    let op = item.opcode();
    let registers: Vec<RegisterId> = item.operands().registers().iter().collect();
    let register = |index: usize| registers.get(index).copied().unwrap_or(RegisterId(0)).0;
    let reg = |index: usize, ty: ArgType| RegisterArg::new(register(index) as RegNum, ty);
    let target = item
        .branch_target()
        .map_or(item.pc().0 as i32, |target| target.0 as i32);
    let payload = item.payload_target().map_or(0, |target| target.0);
    let reference = reference_index(item);
    let invoke_args = || {
        registers
            .iter()
            .map(|register| {
                InsnArg::Reg(RegisterArg::new(register.0 as RegNum, ArgType::unknown()))
            })
            .collect::<Vec<_>>()
    };

    match op {
        NOP => Some(InsnNode::nop()),

        MOVE => Some(InsnNode::move_insn(
            reg(0, ArgType::narrow()),
            InsnArg::Reg(reg(1, ArgType::narrow())),
        )),
        MOVE_FROM16 | MOVE_16 => Some(InsnNode::move_insn(
            reg(0, ArgType::narrow()),
            InsnArg::Reg(reg(1, ArgType::narrow())),
        )),
        MOVE_WIDE | MOVE_WIDE_FROM16 | MOVE_WIDE_16 => Some(InsnNode::move_insn(
            reg(0, ArgType::wide()),
            InsnArg::Reg(reg(1, ArgType::wide())),
        )),
        MOVE_OBJECT | MOVE_OBJECT_FROM16 | MOVE_OBJECT_16 => Some(InsnNode::move_insn(
            reg(0, ArgType::unknown_object()),
            InsnArg::Reg(reg(1, ArgType::unknown_object())),
        )),
        MOVE_RESULT => Some(InsnNode::move_result(reg(0, ArgType::narrow()))),
        MOVE_RESULT_WIDE => Some(InsnNode::move_result(reg(0, ArgType::wide()))),
        MOVE_RESULT_OBJECT => Some(InsnNode::move_result(reg(0, ArgType::unknown_object()))),
        MOVE_EXCEPTION => Some(InsnNode::move_exception(reg(0, ArgType::unknown_object()))),

        RETURN_VOID => Some(InsnNode::return_void()),
        RETURN => Some(InsnNode::return_insn(reg(0, ArgType::narrow()))),
        RETURN_WIDE => Some(InsnNode::return_insn(reg(0, ArgType::wide()))),
        RETURN_OBJECT => Some(InsnNode::return_insn(reg(0, ArgType::unknown_object()))),

        CONST_4 | CONST_16 | CONST | CONST_HIGH16 => Some(InsnNode::const_insn(
            reg(0, ArgType::INT),
            literal_i64(item),
        )),
        CONST_WIDE_16 | CONST_WIDE_32 | CONST_WIDE | CONST_WIDE_HIGH16 => Some(
            InsnNode::const_wide(reg(0, ArgType::LONG), literal_i64(item)),
        ),
        CONST_STRING | CONST_STRING_JUMBO => Some(InsnNode::const_string(
            reg(0, ArgType::unknown_object()),
            reference,
        )),
        CONST_CLASS => Some(InsnNode::const_class(
            reg(0, ArgType::unknown_object()),
            reference,
        )),

        MONITOR_ENTER => Some(InsnNode::monitor_enter(InsnArg::Reg(reg(
            0,
            ArgType::unknown_object(),
        )))),
        MONITOR_EXIT => Some(InsnNode::monitor_exit(InsnArg::Reg(reg(
            0,
            ArgType::unknown_object(),
        )))),

        CHECK_CAST => Some(InsnNode::check_cast(
            InsnArg::Reg(reg(0, ArgType::unknown_object())),
            reference,
        )),
        INSTANCE_OF => Some(InsnNode::instance_of(
            reg(0, ArgType::BOOLEAN),
            InsnArg::Reg(reg(1, ArgType::unknown_object())),
            reference,
        )),

        ARRAY_LENGTH => Some(InsnNode::array_length(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::unknown_object())),
        )),
        NEW_INSTANCE => Some(InsnNode::new_instance(
            reg(0, ArgType::unknown_object()),
            reference,
        )),
        NEW_ARRAY => Some(InsnNode::new_array(
            reg(0, ArgType::unknown_object()),
            InsnArg::Reg(reg(1, ArgType::INT)),
            reference,
        )),

        THROW => Some(InsnNode::throw(InsnArg::Reg(reg(
            0,
            ArgType::unknown_object(),
        )))),

        GOTO | GOTO_16 | GOTO_32 => Some(InsnNode::goto(target)),

        PACKED_SWITCH | SPARSE_SWITCH => Some(InsnNode::switch(
            InsnArg::Reg(reg(0, ArgType::INT)),
            switch_cases(facts, item.pc(), CodeUnitOffset(payload)),
        )),

        CMPL_FLOAT => Some(InsnNode::cmp(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::FLOAT)),
            InsnArg::Reg(reg(2, ArgType::FLOAT)),
            CmpBias::Lt,
        )),
        CMPG_FLOAT => Some(InsnNode::cmp(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::FLOAT)),
            InsnArg::Reg(reg(2, ArgType::FLOAT)),
            CmpBias::Gt,
        )),
        CMPL_DOUBLE => Some(InsnNode::cmp(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::DOUBLE)),
            InsnArg::Reg(reg(2, ArgType::DOUBLE)),
            CmpBias::Lt,
        )),
        CMPG_DOUBLE => Some(InsnNode::cmp(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::DOUBLE)),
            InsnArg::Reg(reg(2, ArgType::DOUBLE)),
            CmpBias::Gt,
        )),
        CMP_LONG => Some(InsnNode::cmp(
            reg(0, ArgType::INT),
            InsnArg::Reg(reg(1, ArgType::LONG)),
            InsnArg::Reg(reg(2, ArgType::LONG)),
            CmpBias::None,
        )),

        IF_EQ..=IF_LE => {
            let if_op = match op {
                IF_EQ => IfOp::Eq,
                IF_NE => IfOp::Ne,
                IF_LT => IfOp::Lt,
                IF_GE => IfOp::Ge,
                IF_GT => IfOp::Gt,
                IF_LE => IfOp::Le,
                _ => IfOp::Eq,
            };
            let operand_type = if matches!(if_op, IfOp::Eq | IfOp::Ne) {
                ArgType::equality_operand()
            } else {
                ArgType::INT
            };
            Some(InsnNode::if_cmp(
                if_op,
                InsnArg::Reg(reg(0, operand_type.clone())),
                InsnArg::Reg(reg(1, operand_type)),
                target,
            ))
        }
        IF_EQZ..=IF_LEZ => {
            let if_op = match op {
                IF_EQZ => IfOp::Eq,
                IF_NEZ => IfOp::Ne,
                IF_LTZ => IfOp::Lt,
                IF_GEZ => IfOp::Ge,
                IF_GTZ => IfOp::Gt,
                IF_LEZ => IfOp::Le,
                _ => IfOp::Eq,
            };
            let operand_type = if matches!(if_op, IfOp::Eq | IfOp::Ne) {
                ArgType::equality_operand()
            } else {
                ArgType::INT
            };
            Some(InsnNode::if_cmp(
                if_op,
                InsnArg::Reg(reg(0, operand_type.clone())),
                InsnArg::lit(0, operand_type),
                target,
            ))
        }

        AGET..=AGET_SHORT => {
            let element = array_element_type(op, false);
            Some(InsnNode::aget(
                reg(0, element),
                InsnArg::Reg(reg(1, ArgType::unknown_object())),
                InsnArg::Reg(reg(2, ArgType::INT)),
            ))
        }
        APUT..=APUT_SHORT => {
            let element = array_element_type(op, true);
            Some(InsnNode::aput(
                InsnArg::Reg(reg(0, element)),
                InsnArg::Reg(reg(1, ArgType::unknown_object())),
                InsnArg::Reg(reg(2, ArgType::INT)),
            ))
        }

        IGET..=IGET_SHORT => Some(InsnNode::iget(
            reg(0, field_element_type(op)),
            InsnArg::Reg(reg(1, ArgType::unknown_object())),
            reference,
        )),
        IPUT..=IPUT_SHORT => Some(InsnNode::iput(
            InsnArg::Reg(reg(0, field_element_type(op))),
            InsnArg::Reg(reg(1, ArgType::unknown_object())),
            reference,
        )),
        SGET..=SGET_SHORT => Some(InsnNode::sget(reg(0, field_element_type(op)), reference)),
        SPUT..=SPUT_SHORT => Some(InsnNode::sput(
            InsnArg::Reg(reg(0, field_element_type(op))),
            reference,
        )),

        INVOKE_VIRTUAL..=INVOKE_INTERFACE => {
            Some(InsnNode::invoke(invoke_type(op), reference, invoke_args()))
        }
        INVOKE_VIRTUAL_RANGE..=INVOKE_INTERFACE_RANGE => {
            Some(InsnNode::invoke(invoke_type(op), reference, invoke_args()))
        }
        INVOKE_POLYMORPHIC | INVOKE_POLYMORPHIC_RANGE => Some(InsnNode::invoke(
            InvokeType::Polymorphic,
            reference,
            invoke_args(),
        )),
        INVOKE_CUSTOM | INVOKE_CUSTOM_RANGE => Some(InsnNode::invoke(
            InvokeType::Custom,
            reference,
            invoke_args(),
        )),

        // Constant method handles and types have no IR value model yet; keep
        // the instruction boundary and leave the value as an explicit gap.
        CONST_METHOD_HANDLE | CONST_METHOD_TYPE => Some(InsnNode::nop()),

        NEG_INT..=INT_TO_SHORT => Some(InsnNode::unary(
            get_unary_op(op),
            reg(0, get_unary_dest_type(op)),
            InsnArg::Reg(reg(1, get_unary_src_type(op))),
        )),

        ADD_INT..=REM_DOUBLE => {
            let (arith_op, arg_type) = decode_arith_3addr_op(op);
            Some(InsnNode::arith(
                arith_op,
                reg(0, arg_type.clone()),
                InsnArg::Reg(reg(1, arg_type.clone())),
                InsnArg::Reg(reg(2, arg_type.clone())),
                arg_type,
            ))
        }
        ADD_INT_2ADDR..=REM_DOUBLE_2ADDR => {
            let (arith_op, arg_type) = decode_arith_2addr_op(op);
            let dest = reg(0, arg_type.clone());
            Some(InsnNode::arith(
                arith_op,
                dest.clone(),
                InsnArg::Reg(dest),
                InsnArg::Reg(reg(1, arg_type.clone())),
                arg_type,
            ))
        }
        ADD_INT_LIT16..=XOR_INT_LIT16 => {
            let (arith_op, _) = decode_arith_lit16_op(op);
            Some(InsnNode::arith(
                arith_op,
                reg(0, ArgType::INT),
                InsnArg::Reg(reg(1, ArgType::INT)),
                InsnArg::lit(literal_i64(item), ArgType::INT),
                ArgType::INT,
            ))
        }
        ADD_INT_LIT8..=USHR_INT_LIT8 => {
            let (arith_op, _) = decode_arith_lit8_op(op);
            Some(InsnNode::arith(
                arith_op,
                reg(0, ArgType::INT),
                InsnArg::Reg(reg(1, ArgType::INT)),
                InsnArg::lit(literal_i64(item), ArgType::INT),
                ArgType::INT,
            ))
        }

        FILL_ARRAY_DATA => Some(InsnNode::fill_array_with_data(
            InsnArg::Reg(reg(0, ArgType::unknown_object())),
            payload,
            fill_array_data(facts, CodeUnitOffset(payload)),
        )),

        FILLED_NEW_ARRAY | FILLED_NEW_ARRAY_RANGE => {
            Some(InsnNode::filled_new_array(reference, invoke_args()))
        }

        _ => Some(InsnNode::nop()),
    }
}

fn reference_index(item: &DecodedInstruction) -> u32 {
    item.references()
        .next()
        .map_or(0, |reference| match reference.target {
            crate::raw::OperandReference::Method(id) => id.0,
            crate::raw::OperandReference::Field(id) => id.0,
            crate::raw::OperandReference::String(id) => id.0,
            crate::raw::OperandReference::Type(id) => id.0,
            crate::raw::OperandReference::Proto(id) => id.0,
            crate::raw::OperandReference::CallSite(id) => id.0,
            crate::raw::OperandReference::MethodHandle(id) => id.0,
        })
}

fn literal_i64(item: &DecodedInstruction) -> i64 {
    match item.operands().literal() {
        Some(RawLiteral::Bits32(bits)) => i64::from(bits as i32),
        Some(RawLiteral::Bits64(bits)) => bits as i64,
        Some(RawLiteral::Signed8(value)) => i64::from(value),
        Some(RawLiteral::Signed16(value)) => i64::from(value),
        None => 0,
    }
}

fn invoke_type(op: u8) -> InvokeType {
    use opcode::*;
    match op {
        INVOKE_VIRTUAL | INVOKE_VIRTUAL_RANGE => InvokeType::Virtual,
        INVOKE_SUPER | INVOKE_SUPER_RANGE => InvokeType::Super,
        INVOKE_DIRECT | INVOKE_DIRECT_RANGE => InvokeType::Direct,
        INVOKE_STATIC | INVOKE_STATIC_RANGE => InvokeType::Static,
        INVOKE_INTERFACE | INVOKE_INTERFACE_RANGE => InvokeType::Interface,
        _ => InvokeType::Virtual,
    }
}

fn array_element_type(op: u8, put: bool) -> ArgType {
    use opcode::*;
    if put {
        match op {
            APUT_WIDE => ArgType::LONG,
            APUT_OBJECT => ArgType::unknown_object(),
            APUT_BOOLEAN => ArgType::BOOLEAN,
            APUT_BYTE => ArgType::BYTE,
            APUT_CHAR => ArgType::CHAR,
            APUT_SHORT => ArgType::SHORT,
            _ => ArgType::INT,
        }
    } else {
        match op {
            AGET_WIDE => ArgType::LONG,
            AGET_OBJECT => ArgType::unknown_object(),
            AGET_BOOLEAN => ArgType::BOOLEAN,
            AGET_BYTE => ArgType::BYTE,
            AGET_CHAR => ArgType::CHAR,
            AGET_SHORT => ArgType::SHORT,
            _ => ArgType::INT,
        }
    }
}

fn field_element_type(op: u8) -> ArgType {
    use opcode::*;
    match op {
        IGET_WIDE | IPUT_WIDE | SGET_WIDE | SPUT_WIDE => ArgType::LONG,
        IGET_OBJECT | IPUT_OBJECT | SGET_OBJECT | SPUT_OBJECT => ArgType::unknown_object(),
        IGET_BOOLEAN | IPUT_BOOLEAN | SGET_BOOLEAN | SPUT_BOOLEAN => ArgType::BOOLEAN,
        IGET_BYTE | IPUT_BYTE | SGET_BYTE | SPUT_BYTE => ArgType::BYTE,
        IGET_CHAR | IPUT_CHAR | SGET_CHAR | SPUT_CHAR => ArgType::CHAR,
        IGET_SHORT | IPUT_SHORT | SGET_SHORT | SPUT_SHORT => ArgType::SHORT,
        _ => ArgType::INT,
    }
}

fn switch_cases(
    facts: &DecodedMethod<'_>,
    instruction_pc: CodeUnitOffset,
    payload: CodeUnitOffset,
) -> Vec<(i32, i32)> {
    let Ok(Some(cases)) = facts.switch_cases_at(payload) else {
        return Vec::new();
    };
    cases
        .map(|case| {
            (
                case.key as i32,
                (instruction_pc.0 as i32).wrapping_add(case.offset.0),
            )
        })
        .collect()
}

fn fill_array_data(facts: &DecodedMethod<'_>, payload: CodeUnitOffset) -> Option<FillArrayData> {
    let Ok(Some(array)) = facts.array_payload_at(payload) else {
        return None;
    };
    let width = array.element_width().0;
    let mut data = Vec::with_capacity(width.saturating_mul(array.element_count() as usize));
    for element in array.elements() {
        for byte in 0..width {
            data.push((element.bits >> (byte * 8)) as u8);
        }
    }
    Some(FillArrayData::new(
        array.element_width().0 as u16,
        array.element_count(),
        data,
    ))
}

fn decode_arith_2addr_op(op: u8) -> (ArithOp, ArgType) {
    use opcode::*;
    match op {
        ADD_INT_2ADDR => (ArithOp::Add, ArgType::INT),
        SUB_INT_2ADDR => (ArithOp::Sub, ArgType::INT),
        MUL_INT_2ADDR => (ArithOp::Mul, ArgType::INT),
        DIV_INT_2ADDR => (ArithOp::Div, ArgType::INT),
        REM_INT_2ADDR => (ArithOp::Rem, ArgType::INT),
        AND_INT_2ADDR => (ArithOp::And, ArgType::INT),
        OR_INT_2ADDR => (ArithOp::Or, ArgType::INT),
        XOR_INT_2ADDR => (ArithOp::Xor, ArgType::INT),
        SHL_INT_2ADDR => (ArithOp::Shl, ArgType::INT),
        SHR_INT_2ADDR => (ArithOp::Shr, ArgType::INT),
        USHR_INT_2ADDR => (ArithOp::Ushr, ArgType::INT),
        ADD_LONG_2ADDR => (ArithOp::Add, ArgType::LONG),
        SUB_LONG_2ADDR => (ArithOp::Sub, ArgType::LONG),
        MUL_LONG_2ADDR => (ArithOp::Mul, ArgType::LONG),
        DIV_LONG_2ADDR => (ArithOp::Div, ArgType::LONG),
        REM_LONG_2ADDR => (ArithOp::Rem, ArgType::LONG),
        AND_LONG_2ADDR => (ArithOp::And, ArgType::LONG),
        OR_LONG_2ADDR => (ArithOp::Or, ArgType::LONG),
        XOR_LONG_2ADDR => (ArithOp::Xor, ArgType::LONG),
        SHL_LONG_2ADDR => (ArithOp::Shl, ArgType::LONG),
        SHR_LONG_2ADDR => (ArithOp::Shr, ArgType::LONG),
        USHR_LONG_2ADDR => (ArithOp::Ushr, ArgType::LONG),
        ADD_FLOAT_2ADDR => (ArithOp::Add, ArgType::FLOAT),
        SUB_FLOAT_2ADDR => (ArithOp::Sub, ArgType::FLOAT),
        MUL_FLOAT_2ADDR => (ArithOp::Mul, ArgType::FLOAT),
        DIV_FLOAT_2ADDR => (ArithOp::Div, ArgType::FLOAT),
        REM_FLOAT_2ADDR => (ArithOp::Rem, ArgType::FLOAT),
        ADD_DOUBLE_2ADDR => (ArithOp::Add, ArgType::DOUBLE),
        SUB_DOUBLE_2ADDR => (ArithOp::Sub, ArgType::DOUBLE),
        MUL_DOUBLE_2ADDR => (ArithOp::Mul, ArgType::DOUBLE),
        DIV_DOUBLE_2ADDR => (ArithOp::Div, ArgType::DOUBLE),
        REM_DOUBLE_2ADDR => (ArithOp::Rem, ArgType::DOUBLE),
        _ => (ArithOp::Add, ArgType::INT),
    }
}

fn decode_arith_3addr_op(op: u8) -> (ArithOp, ArgType) {
    use opcode::*;
    match op {
        ADD_INT => (ArithOp::Add, ArgType::INT),
        SUB_INT => (ArithOp::Sub, ArgType::INT),
        MUL_INT => (ArithOp::Mul, ArgType::INT),
        DIV_INT => (ArithOp::Div, ArgType::INT),
        REM_INT => (ArithOp::Rem, ArgType::INT),
        AND_INT => (ArithOp::And, ArgType::INT),
        OR_INT => (ArithOp::Or, ArgType::INT),
        XOR_INT => (ArithOp::Xor, ArgType::INT),
        SHL_INT => (ArithOp::Shl, ArgType::INT),
        SHR_INT => (ArithOp::Shr, ArgType::INT),
        USHR_INT => (ArithOp::Ushr, ArgType::INT),
        ADD_LONG => (ArithOp::Add, ArgType::LONG),
        SUB_LONG => (ArithOp::Sub, ArgType::LONG),
        MUL_LONG => (ArithOp::Mul, ArgType::LONG),
        DIV_LONG => (ArithOp::Div, ArgType::LONG),
        REM_LONG => (ArithOp::Rem, ArgType::LONG),
        AND_LONG => (ArithOp::And, ArgType::LONG),
        OR_LONG => (ArithOp::Or, ArgType::LONG),
        XOR_LONG => (ArithOp::Xor, ArgType::LONG),
        SHL_LONG => (ArithOp::Shl, ArgType::LONG),
        SHR_LONG => (ArithOp::Shr, ArgType::LONG),
        USHR_LONG => (ArithOp::Ushr, ArgType::LONG),
        ADD_FLOAT => (ArithOp::Add, ArgType::FLOAT),
        SUB_FLOAT => (ArithOp::Sub, ArgType::FLOAT),
        MUL_FLOAT => (ArithOp::Mul, ArgType::FLOAT),
        DIV_FLOAT => (ArithOp::Div, ArgType::FLOAT),
        REM_FLOAT => (ArithOp::Rem, ArgType::FLOAT),
        ADD_DOUBLE => (ArithOp::Add, ArgType::DOUBLE),
        SUB_DOUBLE => (ArithOp::Sub, ArgType::DOUBLE),
        MUL_DOUBLE => (ArithOp::Mul, ArgType::DOUBLE),
        DIV_DOUBLE => (ArithOp::Div, ArgType::DOUBLE),
        REM_DOUBLE => (ArithOp::Rem, ArgType::DOUBLE),
        _ => (ArithOp::Add, ArgType::INT),
    }
}

fn decode_arith_lit16_op(op: u8) -> (ArithOp, ArgType) {
    use opcode::*;
    match op {
        ADD_INT_LIT16 => (ArithOp::Add, ArgType::INT),
        RSUB_INT => (ArithOp::Rsub, ArgType::INT),
        MUL_INT_LIT16 => (ArithOp::Mul, ArgType::INT),
        DIV_INT_LIT16 => (ArithOp::Div, ArgType::INT),
        REM_INT_LIT16 => (ArithOp::Rem, ArgType::INT),
        AND_INT_LIT16 => (ArithOp::And, ArgType::INT),
        OR_INT_LIT16 => (ArithOp::Or, ArgType::INT),
        XOR_INT_LIT16 => (ArithOp::Xor, ArgType::INT),
        _ => (ArithOp::Add, ArgType::INT),
    }
}

fn decode_arith_lit8_op(op: u8) -> (ArithOp, ArgType) {
    use opcode::*;
    match op {
        ADD_INT_LIT8 => (ArithOp::Add, ArgType::INT),
        RSUB_INT_LIT8 => (ArithOp::Rsub, ArgType::INT),
        MUL_INT_LIT8 => (ArithOp::Mul, ArgType::INT),
        DIV_INT_LIT8 => (ArithOp::Div, ArgType::INT),
        REM_INT_LIT8 => (ArithOp::Rem, ArgType::INT),
        AND_INT_LIT8 => (ArithOp::And, ArgType::INT),
        OR_INT_LIT8 => (ArithOp::Or, ArgType::INT),
        XOR_INT_LIT8 => (ArithOp::Xor, ArgType::INT),
        SHL_INT_LIT8 => (ArithOp::Shl, ArgType::INT),
        SHR_INT_LIT8 => (ArithOp::Shr, ArgType::INT),
        USHR_INT_LIT8 => (ArithOp::Ushr, ArgType::INT),
        _ => (ArithOp::Add, ArgType::INT),
    }
}

fn get_unary_op(op: u8) -> UnaryOp {
    use opcode::*;
    match op {
        NEG_INT | NEG_LONG | NEG_FLOAT | NEG_DOUBLE => UnaryOp::Neg,
        NOT_INT | NOT_LONG => UnaryOp::Not,
        INT_TO_LONG => UnaryOp::IntToLong,
        INT_TO_FLOAT => UnaryOp::IntToFloat,
        INT_TO_DOUBLE => UnaryOp::IntToDouble,
        LONG_TO_INT => UnaryOp::LongToInt,
        LONG_TO_FLOAT => UnaryOp::LongToFloat,
        LONG_TO_DOUBLE => UnaryOp::LongToDouble,
        FLOAT_TO_INT => UnaryOp::FloatToInt,
        FLOAT_TO_LONG => UnaryOp::FloatToLong,
        FLOAT_TO_DOUBLE => UnaryOp::FloatToDouble,
        DOUBLE_TO_INT => UnaryOp::DoubleToInt,
        DOUBLE_TO_LONG => UnaryOp::DoubleToLong,
        DOUBLE_TO_FLOAT => UnaryOp::DoubleToFloat,
        INT_TO_BYTE => UnaryOp::IntToByte,
        INT_TO_CHAR => UnaryOp::IntToChar,
        INT_TO_SHORT => UnaryOp::IntToShort,
        _ => UnaryOp::Neg,
    }
}

fn get_unary_dest_type(op: u8) -> ArgType {
    use opcode::*;
    match op {
        NEG_INT | NOT_INT | LONG_TO_INT | FLOAT_TO_INT | DOUBLE_TO_INT => ArgType::INT,
        NEG_LONG | NOT_LONG | INT_TO_LONG | FLOAT_TO_LONG | DOUBLE_TO_LONG => ArgType::LONG,
        NEG_FLOAT | INT_TO_FLOAT | LONG_TO_FLOAT | DOUBLE_TO_FLOAT => ArgType::FLOAT,
        NEG_DOUBLE | INT_TO_DOUBLE | LONG_TO_DOUBLE | FLOAT_TO_DOUBLE => ArgType::DOUBLE,
        INT_TO_BYTE => ArgType::BYTE,
        INT_TO_CHAR => ArgType::CHAR,
        INT_TO_SHORT => ArgType::SHORT,
        _ => ArgType::INT,
    }
}

fn get_unary_src_type(op: u8) -> ArgType {
    use opcode::*;
    match op {
        NEG_INT | NOT_INT | INT_TO_LONG | INT_TO_FLOAT | INT_TO_DOUBLE | INT_TO_BYTE
        | INT_TO_CHAR | INT_TO_SHORT => ArgType::INT,
        NEG_LONG | NOT_LONG | LONG_TO_INT | LONG_TO_FLOAT | LONG_TO_DOUBLE => ArgType::LONG,
        NEG_FLOAT | FLOAT_TO_INT | FLOAT_TO_LONG | FLOAT_TO_DOUBLE => ArgType::FLOAT,
        NEG_DOUBLE | DOUBLE_TO_INT | DOUBLE_TO_LONG | DOUBLE_TO_FLOAT => ArgType::DOUBLE,
        _ => ArgType::INT,
    }
}
