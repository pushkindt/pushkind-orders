import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent as ReactMouseEvent } from "react";

export type DropdownMultiSelectOption = {
  value: string;
  label: string;
  details?: string[];
};

type DropdownMultiSelectProps = {
  id?: string;
  options: DropdownMultiSelectOption[];
  selectedValues: string[];
  onChange: (values: string[]) => void;
  placeholder?: string;
  searchPlaceholder?: string;
  emptyResultsLabel?: string;
  className?: string;
  menuHeightClassName?: string;
  clearable?: boolean;
  clearLabel?: string;
};

export function DropdownMultiSelect({
  id,
  options,
  selectedValues,
  onChange,
  placeholder = "Ничего не выбрано",
  searchPlaceholder = "Фильтр",
  emptyResultsLabel = "Ничего не найдено",
  className,
  menuHeightClassName,
  clearable = false,
  clearLabel = "Очистить выбор",
}: DropdownMultiSelectProps) {
  const rootRef = useRef<HTMLDivElement | null>(null);
  const [open, setOpen] = useState(false);
  const [query, setQuery] = useState("");
  const menuId = id ? `${id}-menu` : undefined;

  useEffect(() => {
    if (!open) {
      return;
    }

    const handlePointerDown = (event: MouseEvent) => {
      if (
        rootRef.current != null &&
        event.target instanceof Node &&
        !rootRef.current.contains(event.target)
      ) {
        setOpen(false);
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => {
      document.removeEventListener("mousedown", handlePointerDown);
    };
  }, [open]);

  const normalizedQuery = query.trim().toLowerCase();

  const filteredOptions = useMemo(() => {
    if (normalizedQuery === "") {
      return options;
    }

    return options.filter((option) => {
      const haystack = [option.label, ...(option.details ?? [])]
        .join(" ")
        .toLowerCase();
      return haystack.includes(normalizedQuery);
    });
  }, [normalizedQuery, options]);

  const selectedOptions = useMemo(
    () => options.filter((option) => selectedValues.includes(option.value)),
    [options, selectedValues],
  );

  const toggleValue = (value: string) => {
    onChange(
      selectedValues.includes(value)
        ? selectedValues.filter((item) => item !== value)
        : [...selectedValues, value],
    );
  };

  const clearSelection = (event: ReactMouseEvent<HTMLButtonElement>) => {
    event.stopPropagation();
    onChange([]);
  };

  return (
    <div
      className={`dropdown auth-dropdown-multiselect ${className ?? ""}`.trim()}
      ref={rootRef}
    >
      <div className="form-control auth-dropdown-multiselect-surface">
        <button
          id={id}
          type="button"
          className="auth-dropdown-multiselect-toggle"
          onClick={() => setOpen((current) => !current)}
          aria-expanded={open}
          aria-controls={menuId}
          aria-haspopup="listbox"
        >
          <span className="auth-dropdown-multiselect-value">
            {selectedOptions.length > 0 ? (
              selectedOptions.map((option) => (
                <span
                  key={`selected-${option.value}`}
                  className="auth-multiselect-chip"
                >
                  {option.label}
                </span>
              ))
            ) : (
              <span className="auth-multiselect-placeholder">
                {placeholder}
              </span>
            )}
          </span>
        </button>
        <span className="auth-dropdown-multiselect-actions">
          {clearable && selectedValues.length > 0 ? (
            <button
              type="button"
              className="auth-dropdown-multiselect-clear"
              title={clearLabel}
              aria-label={clearLabel}
              onClick={clearSelection}
            >
              <i className="bi bi-x-lg" />
            </button>
          ) : null}
          <span
            className="auth-dropdown-multiselect-chevron"
            aria-hidden="true"
          >
            <i className={`bi ${open ? "bi-chevron-up" : "bi-chevron-down"}`} />
          </span>
        </span>
      </div>
      {open ? (
        <div
          id={menuId}
          className="dropdown-menu show w-100 p-0 auth-dropdown-multiselect-menu"
        >
          <div className="p-2 border-bottom">
            <input
              autoFocus
              type="search"
              className="form-control form-control-sm"
              placeholder={searchPlaceholder}
              value={query}
              onChange={(event) => setQuery(event.currentTarget.value)}
            />
          </div>
          <div
            className={`auth-dropdown-multiselect-options ${menuHeightClassName ?? ""}`.trim()}
          >
            {filteredOptions.length > 0 ? (
              filteredOptions.map((option) => {
                const selected = selectedValues.includes(option.value);

                return (
                  <button
                    key={option.value}
                    type="button"
                    className={`dropdown-item auth-dropdown-multiselect-option ${selected ? "active" : ""}`}
                    onClick={() => toggleValue(option.value)}
                  >
                    <strong>{option.label}</strong>
                    {option.details != null && option.details.length > 0 ? (
                      <small className="d-block text-muted">
                        {option.details.join(" ")}
                      </small>
                    ) : null}
                  </button>
                );
              })
            ) : (
              <div className="px-3 py-2 text-muted small">
                {emptyResultsLabel}
              </div>
            )}
          </div>
        </div>
      ) : null}
    </div>
  );
}
