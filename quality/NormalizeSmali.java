import java.io.IOException;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.zip.Adler32;
import org.jf.dexlib2.Opcodes;
import org.jf.dexlib2.dexbacked.DexBackedDexFile;
import org.jf.dexlib2.dexbacked.instruction.DexBackedInstruction;
import org.jf.dexlib2.iface.ClassDef;
import org.jf.dexlib2.iface.Method;
import org.jf.dexlib2.iface.MethodImplementation;
import org.jf.dexlib2.iface.instruction.Instruction;
import org.jf.dexlib2.iface.instruction.VariableRegisterInstruction;

/** Fixture preparation only: correct outgoing-word metadata for all invoke kinds. */
public final class NormalizeSmali {
    public static void main(String[] args) throws Exception {
        if (args.length != 2) throw new IllegalArgumentException("assembled input and output paths required");
        Path input = Path.of(args[0]);
        long size = Files.size(input);
        if (size < 112 || size > Integer.MAX_VALUE) throw new IOException("DEX file exceeds the byte-array platform range");
        byte[] bytes = Files.readAllBytes(input);
        DexBackedDexFile dex = new DexBackedDexFile(Opcodes.forApi(28), bytes);
        ByteBuffer output = ByteBuffer.wrap(bytes).order(ByteOrder.LITTLE_ENDIAN);
        int changed = 0;
        for (ClassDef owner : dex.getClasses()) {
            for (Method method : owner.getMethods()) {
                MethodImplementation implementation = method.getImplementation();
                if (implementation == null) continue;
                Integer start = null;
                int required = 0;
                for (Instruction instruction : implementation.getInstructions()) {
                    if (start == null) start = ((DexBackedInstruction) instruction).instructionStart;
                    if (instruction.getOpcode().name.startsWith("invoke-")
                            && instruction instanceof VariableRegisterInstruction) {
                        required = Math.max(required, ((VariableRegisterInstruction) instruction).getRegisterCount());
                    }
                }
                if (start == null) continue;
                // A regular DEX code_item has its instructions after 16 header bytes.
                long header = (long) start + dex.getBaseDataOffset() - 16;
                if (header < 0 || header + 16 > bytes.length || required > 0xffff) {
                    throw new IOException("code_item metadata exceeds its format range");
                }
                int offset = (int) header + 4;
                int current = Short.toUnsignedInt(output.getShort(offset));
                if (current < required) {
                    output.putShort(offset, (short) required);
                    changed++;
                }
            }
        }
        // Construct the required DEX integrity fields; this is not a checksum gate.
        MessageDigest signature = MessageDigest.getInstance("SHA-1");
        signature.update(bytes, 32, bytes.length - 32);
        System.arraycopy(signature.digest(), 0, bytes, 12, 20);
        Adler32 checksum = new Adler32();
        checksum.update(bytes, 12, bytes.length - 12);
        output.putInt(8, (int) checksum.getValue());
        Files.write(Path.of(args[1]), bytes);
        System.out.println("Normalized outgoing-word metadata in " + changed + " code items; instructions and references retained.");
    }
}
