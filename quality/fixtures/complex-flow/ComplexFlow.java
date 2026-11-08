package fixture;

/** Combined fixture: enum state machine, switch, loop, try/finally, ternary. */
public final class ComplexFlow {
    enum State {
        START,
        RUNNING,
        DONE,
        FAILED
    }

    private State state = State.START;
    private int steps;
    private final int[] history = new int[8];

    public State advance(int input) {
        switch (state) {
            case START:
                state = input > 0 ? State.RUNNING : State.FAILED;
                break;
            case RUNNING:
                if (input < 0) {
                    state = State.FAILED;
                } else if (input == 0) {
                    state = State.DONE;
                }
                break;
            case DONE:
            case FAILED:
            default:
                break;
        }
        if (steps < history.length) {
            history[steps] = input;
        }
        steps++;
        return state;
    }

    public int run(int[] inputs) {
        int applied = 0;
        try {
            for (int index = 0; index < inputs.length; index++) {
                if (inputs[index] == Integer.MIN_VALUE) {
                    continue;
                }
                if (state == State.DONE) {
                    break;
                }
                advance(inputs[index]);
                applied++;
            }
        } finally {
            steps = steps + applied;
        }
        return applied;
    }

    public String summarize() {
        return state == State.DONE ? "done after " + steps : state.name() + ":" + steps;
    }
}
