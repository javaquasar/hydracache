import { Component, type ComponentChildren } from "preact";

type State = { failed: boolean };

export class ConsoleErrorBoundary extends Component<{ children: ComponentChildren }, State> {
  override state: State = { failed: false };

  override componentDidCatch(): void {
    this.setState({ failed: true });
  }

  override render({ children }: { children: ComponentChildren }, { failed }: State) {
    if (!failed) return children;
    return (
      <main class="fatal-console-error" role="alert">
        <h1>Management Center could not render trusted data</h1>
        <p>No diagnostic value from the failed render was displayed.</p>
        <a href="./">Reload the read-only console</a>
      </main>
    );
  }
}
