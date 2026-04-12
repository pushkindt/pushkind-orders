type OrderStatusBadgeProps = {
  status: string;
};

export function orderStatusPresentation(status: string): {
  color: string;
  label: string;
} {
  if (status === "Completed") {
    return { color: "success", label: "Завершён" };
  }

  if (status === "Processing") {
    return { color: "primary", label: "В обработке" };
  }

  if (status === "Pending") {
    return { color: "warning", label: "Ожидает" };
  }

  if (status === "Cancelled") {
    return { color: "danger", label: "Отменён" };
  }

  if (status === "Draft") {
    return { color: "secondary", label: "Черновик" };
  }

  return { color: "secondary", label: "Неизвестно" };
}

export function OrderStatusBadge({ status }: OrderStatusBadgeProps) {
  const presentation = orderStatusPresentation(status);

  return (
    <span className={`badge text-bg-${presentation.color}`}>
      {presentation.label}
    </span>
  );
}
