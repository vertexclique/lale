import PageMeta from "../../components/common/PageMeta";
import PageBreadcrumb from "../../components/common/PageBreadCrumb";
import InvoiceMain from "../../components/invoice/InvoiceMain";

export default function ProductList() {
  return (
    <>
      <PageMeta
        title="React.js E-commerce Single Invoice | LALE Prism - React.js Admin Dashboard Template"
        description="This is React.js E-commerce Single Invoice page for LALE Prism - React.js Tailwind CSS Admin Dashboard Template"
      />
      <PageBreadcrumb pageTitle="Single Invoice" />
      <InvoiceMain />
    </>
  );
}
