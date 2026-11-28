package fixture;

import java.io.InterruptedIOException;

public final class GuardedSourceOrder {
    public static Object collect(Object plan, int mode) {
        try {
            Object first = null;
            Object second = null;
            if (plan instanceof Selection) {
                Selection selected = (Selection) plan;
                if (selected.preview()) {
                    first = Effects.prepare(selected);
                }
                if (!Effects.empty(selected.path())) {
                    second = Effects.confirm(selected.path(), mode);
                }
            }
            Object base = Effects.finish(plan);
            return Effects.combine(Effects.combine(first, base), second);
        } catch (InterruptedIOException interrupted) {
            Effects.stopped();
            return null;
        }
    }

    public static Object collectAfterOpening(Object plan, int mode) {
        try {
            Object first = null;
            Object second = null;
            if (plan instanceof Selection) {
                Selection selected = (Selection) plan;
                if (selected.preview()) {
                    first = Effects.open(selected, mode);
                }
                if (!Effects.empty(selected.path())) {
                    second = Effects.confirm(selected.path(), mode);
                }
            }
            Object base = Effects.finish(plan);
            return Effects.combine(Effects.combine(first, base), second);
        } catch (InterruptedIOException interrupted) {
            Effects.stopped();
            return null;
        }
    }

    public static final class Selection {
        public final boolean preview;
        public boolean remote;

        public Selection(boolean preview, boolean remote) {
            this.preview = preview;
            this.remote = remote;
        }

        public boolean preview() {
            return preview;
        }

        public String path() {
            return remote ? "remote" : "";
        }
    }

    public static final class Effects {
        public static String log = "";

        public static boolean empty(String key) {
            return key.isEmpty();
        }

        public static Object prepare(Selection selected) {
            log += "P";
            return "P";
        }

        public static Object open(Selection selected, int mode) throws InterruptedIOException {
            log += "O";
            if (mode < 0) {
                throw new InterruptedIOException();
            }
            selected.remote = true;
            return "O";
        }

        public static Object confirm(String key, int mode) throws InterruptedIOException {
            log += "C";
            if (mode > 0) {
                throw new InterruptedIOException();
            }
            return "C";
        }

        public static Object finish(Object plan) {
            log += "F";
            return "F";
        }

        public static Object combine(Object first, Object second) {
            log += "M";
            return (first == null ? "" : first.toString()) + (second == null ? "" : second.toString());
        }

        public static void stopped() {
            log += "X";
        }
    }
}
