use super::*;

fn attribute(bytes: &[u8], tail: &[u8]) -> Vec<u8> {
    let mut attribute = vec![0, 3, 0, 2];
    attribute.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    attribute.extend_from_slice(bytes);
    attribute.extend_from_slice(tail);
    attribute
}

#[test]
fn reads_wide_signed_increment_and_preserves_original_offsets() {
    let attribute = attribute(
        &[
            0x10, 0xfe, 0xc4, 0x84, 0x01, 0x2c, 0xff, 0x7f, 0xc4, 0x19, 0x10, 0x02, 0xb1,
        ],
        &[0; 4],
    );
    let code = Code::parse_at(&attribute, 301).unwrap();
    let instructions = code.instructions().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(code.class_file_offset(), 309);
    assert_eq!((code.max_stack, code.max_locals), (3, 2));
    assert_eq!(
        instructions
            .iter()
            .map(|insn| insn.offset())
            .collect::<Vec<_>>(),
        [0, 2, 8, 12],
    );
    assert!(matches!(
        instructions[0].operands(),
        Operands::Immediate(-2)
    ));
    assert!(matches!(
        instructions[1].operands(),
        Operands::Increment {
            local: 300,
            value: -129
        }
    ));
    assert_eq!(instructions[1].modified_mnemonic(), Some("iinc"));
    assert!(matches!(instructions[2].operands(), Operands::Local(4098)));
    assert_eq!(instructions[2].modified_mnemonic(), Some("aload"));
    assert_eq!(instructions[3].mnemonic(), "return");
}

#[test]
fn switch_alignment_uses_bytecode_origin_and_lists_signed_keys_and_offsets() {
    let mut bytes = vec![0, 0, 0xaa, 0];
    for word in [19i32, -1, 0, 0, -2] {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes.extend_from_slice(&[0, 0xab, 0, 0]);
    for word in [-25i32, 2, i32::MIN, -1, i32::MAX, 27] {
        bytes.extend_from_slice(&word.to_be_bytes());
    }
    bytes.push(0xb1);
    let attribute = attribute(&bytes, &[0; 4]);
    let code = Code::parse_at(&attribute, 3).unwrap();
    let instructions = code.instructions().collect::<Result<Vec<_>, _>>().unwrap();
    let table = instructions[2];
    let lookup = instructions[4];
    assert_eq!(table.offset(), 2);
    assert_eq!(lookup.offset(), 25);
    assert_eq!(instructions[5].offset(), 52);
    let cases = |insn: Instruction<'_>| {
        let switch = insn.switch().unwrap();
        let cases = switch.cases().collect::<Vec<_>>();
        (switch.default, cases)
    };
    assert_eq!(cases(table), (19, vec![(-1, 0), (0, -2)]));
    assert_eq!(cases(lookup), (-25, vec![(i32::MIN, -1), (i32::MAX, 27)]));
    assert_eq!(table.operand_problem(), None);
    assert_eq!(lookup.operand_problem(), None);
}

#[test]
fn local_operand_and_handler_errors_keep_later_instructions_and_rows() {
    let attribute = attribute(
        &[0xb9, 0, 20, 0, 0, 0xba, 0, 10, 1, 0, 0x12, 5, 0xb1],
        &[0, 2, 0, 5, 0, 1, 0, 0, 0, 2, 0, 5, 0, 13, 0, 12, 0, 0, 0, 0],
    );
    let code = Code::parse(&attribute).unwrap();
    let instructions = code.instructions().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(instructions[0].constant_index(), Some(20));
    assert_eq!(
        instructions[0].operand_problem(),
        Some(OperandProblem::InvokeInterfaceCount)
    );
    assert_eq!(
        instructions[1].operand_problem(),
        Some(OperandProblem::InvokeReservedBytes)
    );
    assert_eq!(instructions[2].constant_index(), Some(5));
    let mut handlers = code.exception_handlers().unwrap();
    assert_eq!(
        handlers.next().unwrap().range_problem(code),
        Some(OperandProblem::ExceptionHandlerRange)
    );
    let handler = handlers.next().unwrap();
    assert_eq!(handler.range_problem(code), None);
    assert_eq!(handler.catch_type, None);
    assert_eq!(code.attributes_bytes().unwrap(), [0, 0]);
}

#[test]
fn unresolved_width_marks_only_the_unknown_suffix() {
    for (bytes, problem) in [
        (&[0, 0xc4, 0xb1][..], DecodeProblem::InvalidWideOpcode(0xb1)),
        (&[0, 0x11, 0x01][..], DecodeProblem::TruncatedInstruction),
        (&[0, 0xcd, 0xb1][..], DecodeProblem::UndefinedOpcode(0xcd)),
        (
            &[0, 0xab, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 0xff, 0xff][..],
            DecodeProblem::InvalidSwitchCount,
        ),
    ] {
        let input = attribute(bytes, &[]);
        let code = Code::parse(&input).unwrap();
        let mut decoded = code.instructions();
        assert_eq!(decoded.next().unwrap().unwrap().mnemonic(), "nop");
        let gap = decoded.next().unwrap().unwrap_err();
        assert_eq!(gap.range, 1..bytes.len() as u16);
        assert_eq!(gap.problem, problem);
        assert!(decoded.next().is_none());
        assert!(code.exception_handlers().is_err());
    }
}

#[test]
fn code_length_limits_reject_before_observing_instruction_bytes() {
    let overflow = [0, 1, 0, 1, 0, 1, 0, 0];
    assert_eq!(
        Code::parse_at(&overflow, 400).unwrap_err(),
        CodeError {
            offset: 404,
            problem: CodeProblem::InvalidCodeLength(65_536)
        },
    );
}

#[test]
fn truncated_code_and_exception_tables_keep_their_readable_prefix() {
    let code = Code::parse(&[0, 1, 0, 1, 0, 0, 0, 5, 0x10, 0xff, 0xb1]).unwrap();
    assert_eq!(code.missing_bytes(), Some(3..5));
    let decoded = code.instructions().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(decoded.len(), 2);
    assert!(matches!(decoded[0].operands(), Operands::Immediate(-1)));
    assert_eq!(decoded[1].mnemonic(), "return");
    assert_eq!(
        code.exception_handlers().err().unwrap().problem,
        CodeProblem::TruncatedCode
    );

    let data = attribute(&[0, 0xb1], &[0, 2, 0, 0, 0, 1, 0, 1, 0, 99, 0]);
    let code = Code::parse_at(&data, 120).unwrap();
    assert_eq!(code.missing_bytes(), None);
    let mut rows = code.exception_handlers().unwrap();
    assert_eq!(
        rows.next(),
        Some(ExceptionHandler {
            start: 0,
            end: 1,
            target: 1,
            catch_type: Some(99)
        })
    );
    assert_eq!(rows.next(), None);
    assert_eq!(
        rows.issue(),
        Some(CodeError {
            offset: 140,
            problem: CodeProblem::TruncatedExceptionTable
        })
    );
    assert_eq!(
        code.attributes_bytes().unwrap_err().problem,
        CodeProblem::TruncatedExceptionTable
    );
}
