import type { ComponentChildren } from "preact";

type PanelProps = {
  id: string;
  title: string;
  description: string;
  aside?: ComponentChildren;
  children: ComponentChildren;
  className?: string;
};

export function Panel({ id, title, description, aside, children, className = "" }: PanelProps) {
  return (
    <section class={`panel ${className}`.trim()} id={id}>
      <div class="section-heading">
        <div>
          <h2>{title}</h2>
          <p>{description}</p>
        </div>
        {aside}
      </div>
      {children}
    </section>
  );
}

type DataTableProps = {
  label: string;
  headings: readonly string[];
  bodyTestId: string;
};

export function DataTable({ label, headings, bodyTestId }: DataTableProps) {
  return (
    <div class="table-scroll" role="region" aria-label={label} tabIndex={0}>
      <table>
        <thead>
          <tr>{headings.map((heading) => <th scope="col" key={heading}>{heading}</th>)}</tr>
        </thead>
        <tbody data-testid={bodyTestId} />
      </table>
    </div>
  );
}

export function TrustNote({ children }: { children: ComponentChildren }) {
  return <span class="trust-note">{children}</span>;
}
