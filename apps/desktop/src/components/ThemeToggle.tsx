import { useTheme } from "../context/ThemeContext";

export function ThemeToggle() {
  const { theme, toggle } = useTheme();
  return (
    <button className="nav-link" onClick={toggle}>
      {theme === "dark" ? "☀ Light mode" : "● Dark mode"}
    </button>
  );
}
