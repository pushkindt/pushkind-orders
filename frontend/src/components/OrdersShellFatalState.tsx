type OrdersShellFatalStateProps = {
  message: string;
};

export function OrdersShellFatalState({ message }: OrdersShellFatalStateProps) {
  return (
    <main className="orders-foundation-shell">
      <section className="orders-foundation-card">
        <p className="orders-foundation-eyebrow">Orders</p>
        <h1 className="h4 mb-3">Не удалось загрузить страницу</h1>
        <p className="text-secondary mb-0">{message}</p>
      </section>
    </main>
  );
}
