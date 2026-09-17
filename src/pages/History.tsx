import { useTranslation } from "react-i18next";
import { PageTitle } from "@/components/ui/Row";

export function History() {
  const { t } = useTranslation();
  return (
    <div>
      <PageTitle>{t("sidebar.history")}</PageTitle>
      <p className="text-muted">{t("history.empty")}</p>
    </div>
  );
}
