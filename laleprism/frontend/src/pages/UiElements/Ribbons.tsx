import PageBreadcrumb from "../../components/common/PageBreadCrumb";
import RibbonExample from "../../components/ui/ribbons";
import PageMeta from "../../components/common/PageMeta";

export default function Ribbons() {
  return (
    <div>
      <PageMeta
        title="React.js List Ribbons | LALE Prism - React.js Admin Dashboard Template"
        description="This is React.js Ribbons page for LALE Prism - React.js Tailwind CSS Admin Dashboard Template"
      />
      <PageBreadcrumb pageTitle="Ribbons" />
      <RibbonExample />
    </div>
  );
}
