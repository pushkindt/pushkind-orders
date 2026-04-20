import { ShellFatalState } from "@pushkind/frontend-shell/ShellFatalState";

type OrdersShellFatalStateProps = {
  message: string;
};

export function OrdersShellFatalState({ message }: OrdersShellFatalStateProps) {
  return (
    <ShellFatalState
      message={message}
      serviceLabel="Orders"
      title="Не удалось загрузить страницу"
      shellClassName="orders-foundation-shell"
      cardClassName="orders-foundation-card"
      eyebrowClassName="orders-foundation-eyebrow"
      titleClassName="h4 mb-3"
    />
  );
}
