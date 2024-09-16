package bare

import (
	"fmt"
	"testing"
)

const exampleSchema = `
   type PublicKey data[128]
   type Time str # ISO 8601

   type Department enum {
     ACCOUNTING
     ADMINISTRATION
     CUSTOMER_SERVICE
     DEVELOPMENT

     # Reserved for the CEO
     JSMITH = 99
   }

   type Address list<str>[4] # street, city, state, country

   type Customer struct {
     name: str
     email: str
     address: Address
     orders: list<struct {
       orderId: i64
       quantity: i32
     }>
     metadata: map<str><data>
   }

   type Employee struct {
     name: str
     email: str
     address: Address
     department: Department
     hireDate: Time
     publicKey: optional<PublicKey>
     metadata: map<str><data>
   }

   type TerminatedEmployee void

   type Person union {Customer | Employee | TerminatedEmployee}
`

func TestSchema(t *testing.T) {
	p1 := SchemaParser("example1.bare", exampleSchema)
	if err := p1.Parse(); err != nil {
		t.Fatal(err)
	}

	str1, err := p1.ToString()
	if err != nil {
		t.Fatal(err)
	}

	p2 := SchemaParser("example2.bare", str1)
	if err := p2.Parse(); err != nil {
		t.Fatal(err)
	}

	str2, err := p2.ToString()
	if err != nil {
		t.Fatal(err)
	}

	if str1 != str2 {
		fmt.Printf("str1: {{{\n%s}}},\nstr2: {{{\n%s}}}", str1, str2)
		t.Fatal("Something got lost...")
	}
}
